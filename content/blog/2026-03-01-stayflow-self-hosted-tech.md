---
title: "Supabase脱却: StayFlowの自前認証移行 — SQLite + JWT + Resend"
date: "2026-03-01"
description: "StayFlowをSupabase依存からSQLite + JWT + Resendによる自前認証に移行した技術設計と実装の詳細。"
tags: ["StayFlow", "認証", "SQLite", "tech"]
---

## なぜSupabaseから離れるのか

StayFlowは宿泊施設向けPMSとして500+施設が利用中だ。Supabaseは立ち上げ期には最適だったが、以下の問題が顕在化した。

1. **RLSの罠**: `anon`キーでリクエストすると204が返るが実際はブロックされている。`Prefer: return=representation`ヘッダを付けないとサイレントに失敗する
2. **コスト**: 月$25のProプランが施設数に比例して増加。SQLiteなら$0
3. **レイテンシ**: Tokyo regionでもSupabase経由で80-120ms。SQLite WALモードなら1ms以下

移行先はRust (axum) + SQLite + JWT + Resend。認証はマジックリンク方式を継続する。

## 認証フロー設計

```
[ユーザー] ---(1) POST /auth/login {email}---> [axum API]
                                                    |
                                          (2) JWT生成 (exp: 15min)
                                          (3) magic_links テーブルに保存
                                                    |
                                          (4) Resend API でメール送信
                                                    |
[ユーザー] <------- メール受信 ----------------+
    |
    +---(5) GET /auth/verify?token=xxx---> [axum API]
                                               |
                                     (6) magic_links から検索・検証
                                     (7) sessions テーブルにセッション作成
                                     (8) access_token (15min) + refresh_token (30d) 発行
                                               |
[ユーザー] <--- Set-Cookie: refresh_token ---+
           <--- JSON: { access_token } ------+
```

ポイントは、access tokenはメモリ（JavaScript変数）に保持し、refresh tokenはHttpOnly cookieに格納する点だ。XSS耐性とCSRF耐性を両立させる。

## SQLiteスキーマとJWT実装

```sql
-- migrations/001_auth.sql
CREATE TABLE users (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    email TEXT UNIQUE NOT NULL,
    facility_id TEXT REFERENCES facilities(id),
    role TEXT NOT NULL DEFAULT 'staff',  -- 'owner' | 'manager' | 'staff'
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE magic_links (
    token TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id),
    refresh_token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_magic_links_email ON magic_links(email);
CREATE INDEX idx_sessions_user ON sessions(user_id);
```

SQLiteはWALモードで運用し、同時読み取りを許可する。`PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;`を接続時に設定。

JWT生成にはjsonwebtoken crateを使用する。

```rust
// src/auth/jwt.rs
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,         // user_id
    pub email: String,
    pub role: String,
    pub facility_id: Option<String>,
    pub exp: usize,          // Unix timestamp
    pub iat: usize,
}

pub fn create_access_token(user: &User, secret: &[u8]) -> Result<String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        facility_id: user.facility_id.clone(),
        exp: now + 900,  // 15分
        iat: now,
    };
    Ok(encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))?)
}

pub fn create_refresh_token() -> String {
    // 256-bit cryptographically secure random token
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    hex::encode(bytes)
}
```

refresh tokenはDBにSHA256ハッシュとして保存し、平文は保持しない。トークンローテーション（使用ごとに新しいrefresh tokenを発行）で盗用を検知する。

## Resendによるマジックリンク送信

Resend APIはシンプルなREST。SDKは使わず、reqwestで直接呼ぶ。テンプレートはHTML文字列をRust側で`format!`生成する。

リトライは指数バックオフで3回まで。Resendの429レートリミット（100通/秒）に対しては、`tower::limit::RateLimit`ミドルウェアをログインエンドポイントに適用し、IPあたり1通/30秒に制限している。

## Stripe Webhook冪等性

StayFlowはStripeでサブスクリプション課金（Starter: 2,900円/月、Pro: 7,900円/月）を処理する。Webhookの重複配信に対する冪等性設計は以下の通り。

```rust
// src/billing/webhook.rs
pub async fn handle_webhook(
    State(db): State<SqlitePool>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode> {
    let sig = headers.get("stripe-signature")
        .ok_or(AppError::Unauthorized)?;
    let event = stripe::Webhook::construct_event(
        &String::from_utf8_lossy(&body), sig.to_str()?, &WEBHOOK_SECRET
    )?;

    // 冪等性: event.id で重複チェック
    let already = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM webhook_events WHERE event_id = ?)"
    ).bind(&event.id).fetch_one(&db).await?;

    if already {
        tracing::info!(event_id = %event.id, "Duplicate webhook, skipping");
        return Ok(StatusCode::OK);  // Stripeには常に200を返す
    }

    // トランザクション内でイベント処理 + event_id記録
    let mut tx = db.begin().await?;
    match event.type_ {
        EventType::CustomerSubscriptionUpdated => {
            update_subscription(&mut tx, &event).await?;
        }
        EventType::InvoicePaymentFailed => {
            handle_payment_failure(&mut tx, &event).await?;
        }
        _ => {}
    }
    sqlx::query("INSERT INTO webhook_events (event_id, processed_at) VALUES (?, datetime('now'))")
        .bind(&event.id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::OK)
}
```

`webhook_events`テーブルは30日でTTL削除するcronを回す。イベント処理とID記録が同一トランザクションなので、処理途中のクラッシュでも冪等性が保たれる。

## Docker Self-Hosting

オンプレミス需要に応えるため、単一のDockerイメージで完結する設計にした。SQLiteファイルをVolumeマウントするだけで永続化できる。

環境変数は`JWT_SECRET`、`RESEND_API_KEY`、`STRIPE_SECRET_KEY`、`STRIPE_WEBHOOK_SECRET`の4つのみ。Supabase時代の12個から大幅に削減された。

## 移行結果

| 指標 | Supabase時代 | SQLite移行後 |
|------|-------------|-------------|
| 認証レイテンシ (p50) | 95ms | 2ms |
| インフラ月額 | $25+ | $0 (Fly.io shared) |
| 環境変数数 | 12 | 4 |
| デプロイ時間 | N/A | 45秒 (fly deploy) |
| セルフホスト | 不可 | docker compose up |

Supabaseは優れたプロダクトだが、500施設を超えたStayFlowには「所有可能なインフラ」が必要だった。SQLite + JWTという枯れた技術の組み合わせが、結果的に最もシンプルで堅牢な解になった。
