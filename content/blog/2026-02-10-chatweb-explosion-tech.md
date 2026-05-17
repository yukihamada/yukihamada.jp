---
title: "chatweb.ai 93コミットの技術内訳 — Rust + Lambda + DynamoDB"
date: "2026-02-10"
description: "1週間で93コミット。文レベル並列TTS、DynamoDBキー設計修正、Amazon Connect統合、PWA対応の技術詳細"
tags: ["chatweb.ai", "Rust", "Lambda", "tech"]
---

> **ストーリー・背景の概要版** → [1週間で93コミット — chatweb.aiが一気に進化した話](/blog/2026-02-10-chatweb-explosion)

## 概要

2026年2月第1週、chatweb.aiに93コミットを投入した。Claude Codeによる並列実装とsubagent活用で、通常なら1ヶ月かかる変更量を1週間に圧縮した。本記事ではその中から技術的に重要な4つの変更を深掘りする。

## 1. 文レベル並列TTS (Sentence-Level Parallel TTS)

従来のTTSは全テキストを一括で音声合成していた。LLMのストリーミング応答と組み合わせると、全文生成完了まで音声再生が始まらない。これを文単位で並列合成に変更した。

```rust
// crates/nanobot-core/src/tts/parallel.rs
use tokio::sync::mpsc;
use futures::stream::FuturesOrdered;

pub async fn stream_tts_parallel(
    sentences: Vec<String>,
    voice: &str,
    tx: mpsc::Sender<Vec<u8>>,
) -> Result<()> {
    let mut futures = FuturesOrdered::new();

    for (i, sentence) in sentences.into_iter().enumerate() {
        let voice = voice.to_string();
        futures.push_back(tokio::spawn(async move {
            let audio = synthesize_sentence(&sentence, &voice).await?;
            Ok::<(usize, Vec<u8>), anyhow::Error>((i, audio))
        }));
    }

    // 順序を保証しつつ、完了したものから送出
    while let Some(result) = futures.next().await {
        let (_, audio) = result??;
        tx.send(audio).await?;
    }
    Ok(())
}
```

`FuturesOrdered` がポイントだ。`FuturesUnordered` だと完了順に返るが、音声は順序が重要なので `FuturesOrdered` を使う。これにより、最初の文の合成完了時点で再生が始まり、体感レイテンシが平均2.8秒→0.9秒に改善した。

ただし並列度に注意が必要で、10文以上を同時にリクエストするとTTS APIのレート制限に当たる。`Semaphore` で同時実行数を4に制限している。

## 2. DynamoDBキー設計の修正

初期設計ではPartition Keyに `user_id`、Sort Keyに `created_at` を使っていた。これだとホットパーティション問題が発生する — アクティブユーザーのパーティションに負荷が集中する。

修正後のキー設計:

```rust
// crates/nanobot-core/src/db/dynamo.rs

/// conversations テーブル
/// PK: conv#{conversation_id}
/// SK: msg#{timestamp}#{ulid}
///
/// user-conversations GSI
/// PK: user#{user_id}
/// SK: updated#{ISO8601}

#[derive(Debug, Serialize)]
struct ConversationItem {
    #[serde(rename = "PK")]
    pk: String, // "conv#01HQXYZ..."

    #[serde(rename = "SK")]
    sk: String, // "msg#2026-02-10T03:21:00Z#01HQABC..."

    user_id: String,
    role: String,    // "user" | "assistant"
    content: String,
    model: String,

    // GSI keys
    #[serde(rename = "GSI1PK")]
    gsi1pk: String, // "user#usr_abc123"

    #[serde(rename = "GSI1SK")]
    gsi1sk: String, // "updated#2026-02-10T03:21:00Z"
}
```

conversation_idにULIDを使うことで、パーティションが自然に分散する。ULIDはタイムスタンプを含むため、range queryでの時系列ソートも可能だ。GSIでユーザー単位の会話一覧取得を実現する。

この変更で、ピーク時のDynamoDB throttling errorが完全に消えた。

## 3. Amazon Connect統合

電話番号からchatweb.aiのAIチャットに接続する機能を追加した。Amazon Connect Contact Flowからの着信をLambdaで受け、WebSocket経由でchatweb.aiのチャットセッションに橋渡しする。

技術的に難しかったのは、Amazon ConnectのStreaming APIとchatweb.aiのSSEストリームの橋渡しだ。Connectは8kHz/16bit PCMで音声を送ってくるが、TTSは24kHz/MP3で返す。リアルタイムでのフォーマット変換が必要だった。

```rust
// crates/nanobot-core/src/connect/bridge.rs
use aws_sdk_connect::types::AudioStream;

async fn bridge_audio(
    connect_stream: AudioStream,
    tts_rx: mpsc::Receiver<Vec<u8>>,
) -> Result<()> {
    // Connect → chatweb.ai: PCM 8kHz → Whisper (16kHz upsampled)
    let transcribe_handle = tokio::spawn(async move {
        let upsampled = resample_pcm(connect_stream, 8000, 16000).await;
        whisper_streaming_transcribe(upsampled).await
    });

    // chatweb.ai → Connect: MP3 24kHz → PCM 8kHz
    let playback_handle = tokio::spawn(async move {
        while let Some(mp3_chunk) = tts_rx.recv().await {
            let pcm = mp3_to_pcm(&mp3_chunk, 8000)?;
            connect_stream_sink.send(pcm).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    tokio::try_join!(transcribe_handle, playback_handle)?;
    Ok(())
}
```

## 4. PWA Service Worker と i18n

PWAのService Workerでは、チャット履歴のオフラインキャッシュを実装した。IndexedDBに最新50会話を保持し、オフライン時でも過去の会話を閲覧できる。

i18n対応は45テストケースで検証している。日本語・英語・中国語(簡体/繁体)・韓国語の5言語に対応し、各言語でのUI文字列とTTS voice idのマッピングをテストする。

特に厄介だったのは、日本語の敬語レベルの切り替えだ。ビジネス用途では「です・ます」調、カジュアル用途では「だ・である」調をLLMのsystem promptで制御する。これをi18nの一部として管理するか、別のレイヤーで管理するかで議論した結果、`locale_config` テーブルに `formality_level` カラムを追加し、言語設定と敬語レベルを独立に管理する設計にした。

## パフォーマンス結果

| メトリクス | Before | After | 改善率 |
|-----------|--------|-------|--------|
| TTS初回再生 | 2.8s | 0.9s | -68% |
| DynamoDB throttle | 23回/日 | 0回/日 | -100% |
| PWAオフライン対応 | なし | 50会話 | -- |
| 対応言語 | 2 | 5 | +150% |

## 振り返り

93コミット中、手動で書いたコードは全体の約20%。残り80%はClaude Codeが生成し、人間がレビューした。特にi18nのテストケース生成とDynamoDBのスキーママイグレーションはAI生成との相性が良い。一方で、Amazon Connect統合のようなAWS固有のエッジケースは人間の経験が必要だった。
