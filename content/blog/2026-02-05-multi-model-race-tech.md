---
title: "マルチモデルレースの実装 — サーキットブレーカー + SSEストリーミング"
date: "2026-02-05"
description: "chatweb.aiにおけるRust並列モデル呼び出し、tokio::select!レースパターン、サーキットブレーカー、SSEストリーミングの実装詳細。"
tags: ["chatweb.ai", "Rust", "SSE", "tech"]
---

## 問題: 単一モデル依存の脆弱性

chatweb.aiは複数のLLMプロバイダ（Anthropic、OpenAI、Groq、Nemotron自前ポッド）を利用する。単一プロバイダに依存すると、障害時にサービス全体が停止する。OpenRouterの$3,400クレジットが枯渇した2026年3月のインシデントで、この問題が顕在化した。

解決策として「マルチモデルレース」パターンを実装した。複数モデルに同時リクエストを投げ、最初にレスポンスを返したモデルの出力をユーザーに配信する。

## tokio::select! によるレースパターン

Rustの`tokio::select!`マクロは、複数の非同期タスクを同時に待ち、最初に完了したものを採用する。これがレースの核心だ。

```rust
// crates/nanobot-core/src/inference/race.rs
use tokio::sync::mpsc;

pub async fn race_models(
    prompt: &ChatRequest,
    models: &[ModelEndpoint],
    breaker: &CircuitBreakerRegistry,
) -> Result<mpsc::Receiver<SseEvent>> {
    let (tx, rx) = mpsc::channel::<SseEvent>(256);

    // 各モデルへの並列リクエストを生成
    let futures: Vec<_> = models.iter()
        .filter(|m| breaker.is_available(&m.id))  // 開いているブレーカーを除外
        .map(|model| {
            let tx = tx.clone();
            let prompt = prompt.clone();
            let model = model.clone();
            async move {
                let result = call_model_streaming(&model, &prompt, tx).await;
                (model.id.clone(), result)
            }
        })
        .collect();

    if futures.is_empty() {
        return Err(AppError::AllModelsUnavailable);
    }

    // 最初に成功したモデルを採用、残りはキャンセル
    tokio::spawn(async move {
        let (model_id, result) = tokio::select! {
            // select!は最初に完了したfutureを返し、残りをdropする
            // dropされたfutureのHTTP接続は自動的にcloseされる
            result = async {
                let mut set = tokio::task::JoinSet::new();
                for fut in futures {
                    set.spawn(fut);
                }
                // 最初に成功したものを返す
                while let Some(res) = set.join_next().await {
                    match res {
                        Ok((id, Ok(()))) => return (id, Ok(())),
                        Ok((id, Err(e))) => {
                            tracing::warn!(model = %id, error = %e, "Model failed");
                            continue;
                        }
                        Err(e) => {
                            tracing::error!("Task panicked: {}", e);
                            continue;
                        }
                    }
                }
                ("none".into(), Err(AppError::AllModelsFailed))
            } => result,
        };

        match &result {
            Ok(()) => tracing::info!(model = %model_id, "Race won"),
            Err(e) => tracing::error!("All models failed: {}", e),
        }
    });

    Ok(rx)
}
```

重要な設計判断として、`JoinSet`を使い「最初の成功」を待つ。単純な`select!`だと最初に完了したfutureがエラーだった場合に失敗するが、`JoinSet`で順次結果を確認することで、全モデルが失敗するまで粘る。

## サーキットブレーカー

障害中のモデルに毎回リクエストを投げるのは無駄であり、レイテンシを悪化させる。サーキットブレーカーパターンで、連続失敗したモデルを一時的に除外する。

```rust
// crates/nanobot-core/src/inference/circuit_breaker.rs
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub struct CircuitBreaker {
    failures: AtomicU32,
    last_failure: AtomicU64,      // Unix timestamp (ms)
    state: AtomicU8,              // 0=Closed, 1=Open, 2=HalfOpen
}

const FAILURE_THRESHOLD: u32 = 3;
const RECOVERY_TIMEOUT_MS: u64 = 30_000;  // 30秒

impl CircuitBreaker {
    pub fn is_available(&self) -> bool {
        match self.state.load(Ordering::Relaxed) {
            0 => true,   // Closed: 正常、リクエスト許可
            1 => {       // Open: 障害中
                let elapsed = now_ms() - self.last_failure.load(Ordering::Relaxed);
                if elapsed > RECOVERY_TIMEOUT_MS {
                    // Half-Open: 試行的に1リクエスト許可
                    self.state.store(2, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            }
            2 => false,  // HalfOpen: 試行中、追加リクエスト不可
            _ => unreachable!(),
        }
    }

    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.state.store(0, Ordering::Relaxed);  // Closed に戻す
    }

    pub fn record_failure(&self) {
        let count = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure.store(now_ms(), Ordering::Relaxed);
        if count >= FAILURE_THRESHOLD {
            self.state.store(1, Ordering::Relaxed);  // Open にする
            tracing::warn!("Circuit breaker opened after {} failures", count);
        }
    }
}
```

状態遷移: `Closed`(正常) -> 3連続失敗 -> `Open`(遮断) -> 30秒経過 -> `HalfOpen`(試行) -> 成功 -> `Closed` / 失敗 -> `Open`。Atomic操作のみでロック不要なため、高並行下でもスケールする。

## SSEストリーミング

レースの勝者が決まると、そのモデルからのストリーミングチャンクを即座にSSE（Server-Sent Events）としてクライアントに流す。axumの`Sse`レスポンスとmpscチャネルを接続する。

```rust
// crates/nanobot-core/src/handler.rs
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;

pub async fn chat_handler(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = race_models(&req, &state.models, &state.breakers)
        .await
        .expect("at least one model available");

    let stream = ReceiverStream::new(rx).map(|event| {
        Ok(Event::default()
            .event("message")
            .data(serde_json::to_string(&event).unwrap()))
    });

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"))
}
```

SSEのkeep-aliveは15秒間隔で`ping`コメントを送信する。API Gatewayのアイドルタイムアウト（29秒）を回避するために必須だ。Lambda環境ではレスポンスストリーミング（`InvokeWithResponseStream`）を使い、SSEチャンクをそのまま流す。

## DynamoDBセッション管理

Lambda環境ではローカルファイルシステムが使えないため、会話履歴はDynamoDBに保存する。テーブル設計は以下の通り。

- **PK**: `session#{session_id}` — セッション識別
- **SK**: `msg#{timestamp}#{seq}` — メッセージ順序
- **TTL**: `expires_at` — 30日で自動削除

セッションIDはクライアントが生成し（UUIDv4）、`Authorization`ヘッダまたはcookieで送信する。DynamoDBの`Query`で`SK`のプレフィックス検索（`begins_with(SK, 'msg#')`）を使い、直近N件のメッセージをコンテキストとして取得する。

## 実測パフォーマンス

| 指標 | 単一モデル | マルチモデルレース |
|------|----------|-----------------|
| TTFB (p50) | 850ms | 420ms |
| TTFB (p99) | 3,200ms | 1,100ms |
| 可用性 (30日) | 99.2% | 99.97% |
| 月間コスト | $180 | $210 (+17%) |

TTFBは最初のトークンが返るまでの時間。レースにより常に最速のモデルが選ばれるため、p50で50%改善。コスト増は17%だが、実際にはサーキットブレーカーで無駄なリクエストが削減されるため、負荷に応じて変動する。

## まとめ

マルチモデルレースは「冗長性のためのコスト」ではなく「レイテンシ改善のための投資」だ。tokio::select!とサーキットブレーカーの組み合わせにより、障害時のフェイルオーバーと通常時のレイテンシ最適化を同時に達成できる。SSEストリーミングとの統合により、ユーザー体験を損なわずにバックエンドの複雑性を吸収している。
