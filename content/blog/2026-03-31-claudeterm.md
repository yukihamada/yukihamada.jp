---
title: "ブラウザから Claude Code が使える「claudeterm」を作った"
date: 2026-03-31
description: "インストール不要でブラウザから Claude Code を操作できる SaaS を Rust + WebSocket で作った記録。マルチユーザー対応、OTP 認証、クレジット制課金まで。"
tags: ["claudeterm", "Claude", "Rust", "WebSocket", "SaaS", "開発記録"]
---

Claude Code（Anthropic の CLI ツール）が便利すぎて、「これブラウザから使えたら最高じゃないか」と思って作ったのが **claudeterm**。

https://term.pasha.run

## 何ができるか

- ブラウザから Claude Code を操作できる
- インストール不要、メールアドレスだけで使える
- 複数セッション管理、会話履歴の永続化
- 8種のプロジェクトテンプレート（Web App / Mobile / Data / DevOps など）
- 自分の API キーを持ち込む（BYOK）か、プラットフォームのクレジットを使う

## 技術構成

| 層 | 技術 |
|---|---|
| Backend | Rust + Axum 0.7 |
| Frontend | バニラ JS（単一 HTML ファイル、677行） |
| DB | SQLite (rusqlite, bundled) |
| リアルタイム | WebSocket + ストリーミング JSON |
| 認証 | メール OTP（Resend） |
| 課金 | Stripe Checkout |
| デプロイ | Fly.io Tokyo + Docker + GitHub Actions |

## 実装で面白かったところ

**モデルの自動ルーティング**。メッセージの構造を解析して effort を自動選択する。キーワードマッチじゃなく「コードブロックがある → high」「60文字未満の疑問文 → low」という構造ベースの判定。

```rust
pub fn route_message(text: &str) -> (&'static str, &'static str) {
    const MODEL: &str = "claude-sonnet-4-6";
    if text.contains("```") || text.contains("diff\n") || text.len() > 400 {
        return (MODEL, "high");
    }
    let trimmed = text.trim();
    if text.len() < 60 && (trimmed.ends_with('?') || trimmed.ends_with('？')) {
        return (MODEL, "low");
    }
    (MODEL, "medium")
}
```

**ユーザー分離**。Linux（Fly.io）では Docker コンテナ内でユーザーごとにサンドボックスディレクトリを切る。macOS では `sandbox-exec` を使って OS レベルで書き込み制限。

**詰まったポイント**：Claude CLI を root で実行すると `--dangerously-skip-permissions` が弾かれる。Fly.io のコンテナはデフォルト root なので、`docker-entrypoint.sh` で `gosu node` に切り替えて解決した。

## macOS アプリ（NOU）

ローカルの Mac で動かしたい人向けに、メニューバーアプリ「NOU」も作った。claudeterm バイナリを内包していて、起動するだけでローカルサーバーが立ち上がる。OTP 不要・クレジット不要・自分の Mac の Claude を使う。

```bash
# リポジトリから NOU.app をビルド
bash nou/build.sh
# → nou/NOU.dmg (3MB) が生成される
```

## オープンソース

https://github.com/yukihamada/claudeterm

Rust + Axum + WebSocket でシンプルに書いたので、自分でホストしたい人はそのまま `fly deploy` で動く。
