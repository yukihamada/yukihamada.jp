# yukihamada.jp

濱田優貴の個人サイト。Rust + Axum + Askama (server-side rendering)、Fly.io でホスト。

[![Deploy](https://github.com/yukihamada/yukihamada.jp/actions/workflows/deploy.yml/badge.svg)](https://github.com/yukihamada/yukihamada.jp/actions/workflows/deploy.yml)

公開先: <https://yukihamada.jp>

## 構成

- **Backend**: Rust + Axum 0.8 + Askama テンプレート、Markdown 記事を `content/blog/` から読み込み
- **Admin terminal**: portable-pty + WebSocket でブラウザから bash セッション (要 ANTHROPIC_API_KEY)
- **Frontend**: 静的アセット (`public/`, `style/`) + 一部 Vite/フロント (`frontend/`)
- **Deploy**: Fly.io app `yukihamada-jp` (region `nrt`, 512MB)

## クイックスタート (ローカル)

```bash
cargo run --release
# http://localhost:8080
```

## 主要ファイル

```
src/main.rs          # Axum サーバ (routes / WS PTY / SSE)
src/blog.rs          # Markdown → HTML (pulldown-cmark)
templates/           # Askama (home / about / blog_list / blog_post / mcp / 404)
content/blog/        # Markdown 記事 (yyyy-mm-dd-slug.md)
public/              # 静的アセット (favicons / images / audio / anime)
style/               # CSS
frontend/            # フロントエンドソース
scripts/             # ビルド・デプロイヘルパー
m5_server.py         # m5 Mac 連携サーバ
Dockerfile  fly.toml # 本番ビルド
.github/workflows/   # deploy.yml (Fly auto-deploy)
```

## デプロイ

`main` への push → GitHub Actions 経由で Fly.io にデプロイ。

```bash
# 手動: fly deploy --remote-only -a yukihamada-jp
```

---

Maintained by [Enabler Inc.](https://enablerdao.com)
