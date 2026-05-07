---
title: "ミセバンAI技術設計 — Rust 4クレート + Fly Postgres + ONNX"
date: "2026-02-18"
description: "ミセバンAIのRust 4クレートアーキテクチャ、ONNXランタイム統合、Raspberry Piクロスコンパイル、macOS公証までの技術詳細。"
tags: ["ミセバンAI", "Rust", "tech"]
---

## アーキテクチャ概要

ミセバンAIは「店舗向けAI番人」として、来客分析・異常検知・売上予測を単一バイナリで提供するプロダクトだ。技術的な特徴は、Rust monorepoの4クレート構成と、ONNX Runtimeによるエッジ推論の両立にある。

### 4クレート構成

設計方針は「関心の分離」と「ターゲット別ビルド」の両立。店舗のRaspberry Pi、macOSデスクトップ、クラウドAPIが同一コードベースから生成される。

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/miseban-core",    # ドメインロジック + ONNXモデル管理
    "crates/miseban-api",     # axum HTTP API + WebSocket
    "crates/miseban-edge",    # Raspberry Pi / エッジデバイス向け
    "crates/miseban-desktop", # macOS / Windows GUI (Tauri)
]

[workspace.dependencies]
axum = "0.7"
ort = "2.0"              # ONNX Runtime bindings
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
```

`miseban-core`がすべての共通ロジックを持ち、他の3クレートは薄いアダプタ層として機能する。これにより、エッジ向けビルドではGUI依存を完全に排除し、バイナリサイズを12MBに抑えている。

## ONNXモデルダウンロードとキャッシュ

モデルファイル（YOLOv8n: 6.3MB、カスタム異常検知: 2.1MB）は初回起動時にダウンロードし、`$XDG_DATA_HOME/miseban/models/`にキャッシュする。SHA256チェックサムで整合性を検証し、破損時は再ダウンロードが走る。

```rust
// crates/miseban-core/src/model_manager.rs
pub async fn ensure_model(spec: &ModelSpec) -> Result<PathBuf> {
    let cache_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("miseban/models");
    fs::create_dir_all(&cache_dir).await?;

    let model_path = cache_dir.join(&spec.filename);
    if model_path.exists() {
        let hash = sha256_file(&model_path).await?;
        if hash == spec.expected_sha256 {
            return Ok(model_path);
        }
        tracing::warn!("Model checksum mismatch, re-downloading");
    }

    // ストリーミングダウンロード（メモリに全載せしない）
    let response = reqwest::get(&spec.url).await?;
    let mut file = tokio::fs::File::create(&model_path).await?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        tokio::io::copy(&mut chunk?.as_ref(), &mut file).await?;
    }
    Ok(model_path)
}
```

ORT (ort crate) のセッション生成は重い処理のため、`Arc<Session>`で共有し、リクエストごとの再生成を回避している。推論は`spawn_blocking`でtokioランタイムをブロックしない設計とした。

## Fly.io Postgres との接続

クラウド側のデータストアにはFly.io Managed Postgresを採用。接続プールは`deadpool-postgres`で管理し、Fly internal DNS（`miseban-db.internal:5432`）経由でWireGuardトンネル越しに接続する。マイグレーションは`refinery`で管理。

エッジデバイスはローカルSQLiteに書き込み、5分間隔でクラウドへ差分同期する。競合解決はLWW（Last Writer Wins）で、タイムスタンプカラムを全テーブルに持たせている。

## Raspberry Pi クロスコンパイル

Raspberry Pi 4（aarch64）向けビルドは`cross`を使う。ONNX Runtimeの動的ライブラリはコンテナ内でビルドされるため、ホスト環境の汚染がない。

```bash
# Raspberry Pi 4 (aarch64) 向けクロスコンパイル
cross build --manifest-path crates/miseban-edge/Cargo.toml \
  --release --target aarch64-unknown-linux-gnu

# ARMv7 (Raspberry Pi 3) 向け
cross build --manifest-path crates/miseban-edge/Cargo.toml \
  --release --target armv7-unknown-linux-gnueabihf

# バイナリサイズ確認（stripで12MB → 8.7MB）
aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/miseban-edge
ls -lh target/aarch64-unknown-linux-gnu/release/miseban-edge
```

注意点として、ORT 2.0はARMv7でNEON SIMDを使うが、一部の古いPi 3ではクラッシュする。`ORT_USE_NEON=0`環境変数でフォールバック可能だ。

## macOS コード署名と公証

macOSデスクトップ版はTauriでビルドし、Apple Notarization APIで公証を通す。CI/CDはGitHub Actionsで自動化済み。

署名には`Developer ID Application`証明書が必要で、Keychain Accessからp12エクスポートしたものをGitHub Secretsに格納。`codesign --deep --force`で全フレームワークに署名後、`xcrun notarytool submit`でAppleに送信する。公証完了まで通常2-5分、`xcrun stapler staple`でチケットを埋め込んで配布する。

## パフォーマンス実測値

| 指標 | Raspberry Pi 4 | M1 Mac | Fly.io (shared-cpu-1x) |
|------|---------------|--------|----------------------|
| YOLOv8n推論 | 180ms/frame | 12ms/frame | 45ms/frame |
| API レイテンシ (p99) | — | — | 23ms |
| メモリ使用量 | 87MB | 120MB | 95MB |
| コールドスタート | 1.2s | 0.4s | 0.8s |

エッジ推論180msは30fpsには届かないが、店舗の来客カウントには5fps程度で十分であり、実用上問題ない。

## まとめ

4クレート分離により「同一ロジック、異なるターゲット」を実現し、ONNXモデルのエッジ配信でクラウド依存を最小化した。Raspberry Piで動くAIが月額0円で店舗を見守る――これがミセバンAIの技術的な核心だ。
