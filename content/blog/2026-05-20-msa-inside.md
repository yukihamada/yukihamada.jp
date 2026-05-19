---
title: "MU Source Access の中身を解剖する — 21リポ、それぞれ何を解いてるか"
date: 2026-05-20
tags: [MU, "MU Source Access", MSA, wearmu, OSS, source-available, Rust, Swift, TypeScript]
description: "昨日 MU Source Access (MSA) を発表した。Tシャツ買うと private リポ21本のソースが読める仕組みだ。じゃあ中身は何なのか。21リポを1段ずつ、何を解いているか・どこで詰まったか・読んで何が得られるかを書く。コードは出さない、要約だけ。"
---

[昨日](/blog/2026-05-19-open-source-stop) **MU Source Access**（MSA）を発表した。21本の private リポのソースを、wearmu のTシャツ買ってくれた人にだけ読めるようにする、というやつだ。

「で、中身は何なの？」と聞かれた。当然だ。買う前に何が手に入るかわからないと買えない。

このエントリでは21リポを1段ずつ、**何を解いてるか / 詰まりどころ / 読んで持ち帰れるもの**を書く。コードは出さない（それは買ってから読んでもらう）。**要約と感想だけ**だ。

> **👉 [先に Tシャツ買って /source に入る](https://wearmu.com/source)**

---

## 1. trio — macOS AIメッセージアシスタント

LINE/Slack/Chatwork/Discord/Telegram の通知を取り込み、AIが要約 + 3パターンの返信プランをカード状に提示。Swift製のMacアプリ + Rust製のクラウドリレー (trio-cloud.fly.dev)。

**詰まりどころ:** LINEのMacアプリは公式CLIなし。`cliclick` で座標叩いて自動送信を実現してるが、ウィンドウメニュー名がOSバージョンで変わる罠あり (詳細はリポの `feedback_line_send.md`)。

**読んで得るもの:** Vision OCR + SwiftPM 配布 (PKG/notarize/staple) の一連の現実的フロー。

## 2. kagi — スマートホーム iOS/Mac Catalyst

Hue / SwitchBot のリモコン統合。Macアプリ化 (Catalyst) で iOS UI そのまま Mac でも動かす。

**詰まりどころ:** AutoFill の落とし穴で AppStore リジェクト経験あり。コミット履歴に retry の試行錯誤が残ってる。

**読んで得るもの:** Mac Catalyst でハマる地雷リスト。iOS設計をMacに持ち込むときの判断基準。

## 3. claudeterm — Terminal-native LLM agent

ターミナルでローカル/リモート両方の LLM を扱う Rust 製エージェント。S3 backup付き。

**詰まりどころ:** rust-openssl の CVE 連発で先週 5バージョン更新。`rust-s3 0.34→0.37` の API破壊変更で `Bucket::new` の返り値が `Box<Bucket>` に変わった件、丸ごと PR にした (#16) ので diff で見られる。

**読んで得るもの:** ターミナル UI で reactive な agent loop を組む実装パターン。

## 4. jitsuflow — 柔術試合フロー記録 (Dart/Flutter)

[JiuFlow iOS](https://jiuflow.com) の姉妹アプリ。試合のテイクダウン/スイープ/サブミッションを時系列記録。

**詰まりどころ:** Cloudflare Worker バックエンド + Dart pubspec で wrangler 4 系の major bump (`wrangler 4.24→4.92`) と vitest 4 の config 衝突で一度詰まった。PR #22 で解決。

**読んで得るもの:** Flutter + Cloudflare Worker のフルスタック構成、競技ドメイン特有の状態モデリング。

## 5. tsugi — 継承 (succession) プロトコルの参考実装

append-only / content-addressed / era stamp / succession_token を全部入れた小さな Rust crate。`bim.house` の沈黙ビジョンの実装側。

**詰まりどころ:** content-addressed 設計と append-only ストレージの両立。content hash 衝突時の era stamp の役割定義に1週間溶かした。

**読んで得るもの:** 「永遠に残せるデータ構造」の素朴な答え。

## 6. hato — 軽量メッセージング実験

Axum + libsql の最小実装。リアルタイム push 試作。

**読んで得るもの:** Rust で realtime backend を 1000行 以内にまとめる例。

## 7. hypernews — news.xyz バックエンド

記事ランキング / コメント / ハイパーリンク収集の Rust + Axum 実装。

**詰まりどころ:** ランキングアルゴリズムを HN 似にしつつ、半減期 (decay) を調整する試行。

**読んで得るもの:** 自前で HN ライクなサイトを立てる際の最小構成。

## 8. pasha — レシート経費管理 iOS

Swift + Vision OCR + Stripe billing。日本の経費精算ワークフローに特化。

**詰まりどころ:** Vision のOCR精度が領収書のレイアウトでばらつく。fastlane で App Store Connect API キー認証だと審査提出ができず、Apple ID 認証に戻した経緯あり (詳細は `feedback_app_store_review_submission`)。

**読んで得るもの:** iOS + Stripe + Vision の最小実装、 fastlane の落とし穴。

## 9. pon — 電子契約・署名 iOS

電子帳簿保存法対応。PDF にメタデータ + 署名 + タイムスタンプ。

**読んで得るもの:** 日本特有の法令対応 (電帳法) を Swift で素直に書くとどうなるか。

## 10. NOU — ローカル LLM サーバ + GUI installer

mac/win/linux の installer 同時生成 (.dmg / .msi / .AppImage)。サーバは Rust、UI は Swift。

**詰まりどころ:** GitHub Actions で 3OSビルドを並列にすると build時間 + cache 戦略で詰まる。

**読んで得るもの:** クロスプラットフォーム installer を CI で自動化するレシピ。

## 11. factlens — 事実検証ツール (Rust)

URL や claim を投げると複数ソースを横断 cross-check して "どこまで本当か" を返す。

**詰まりどころ:** Web fetch の rate limit + agentic な reasoning ループの管理。

**読んで得るもの:** "事実を確かめる" を AI に投げる時の設計。

## 12. gitnote — git-backed note-taking

全ての note 編集が git commit になる。履歴・branch・restore が無料で付いてくる note app。

**読んで得るもの:** 「データベース要らない、bare git で済む」アプリ構造の極端な例。

## 13. flow-anime — アニメシーン記録

「あの続き観たい」をすぐ起動できる Web ツール。HTML + minimal JS。

**読んで得るもの:** Notion でも Obsidian でもない、超軽量の personal index 設計。

## 14. phishguard — 日本企業向けフィッシング訓練 SaaS

Next.js + Prisma + SQLite + Litestream + Stripe。社員に模擬フィッシングメールを送って結果を可視化する MVP。

**詰まりどころ:** Litestream のレプリケーション設定と Fly.io の volume の組み合わせで一度データロスしかけた (recovery 成功)。

**読んで得るもの:** SQLite を本気で本番投入する時の重要点。

## 15. security-scanner — 中小企業 Web セキュリティ診断 SaaS

URL を投げると診断レポート PDF を返す。Stripe 課金 UI 付き、Fly.io デプロイ。

**読んで得るもの:** SaaS MVP を 1000行で立ち上げる構造。

## 16. security-education — サイバーセキュリティ教育プラットフォーム

Next.js 16 + React 19 + Tailwind。無料記事で集客 + 有料講座 + コミュニティ。

**読んで得るもの:** 教育系 SaaS のテンプレ。

## 17. nemotron — Nvidia Nemotron Nano 9B v2 ラッパー

Modal/RunPod に 1コマンド deploy する OpenAI互換 API ラッパー。vLLM 経由で `/v1/chat/completions` 等を実装。

**詰まりどころ:** Modal と RunPod の autoscale 仕様の違いで、cold start を制御する書き方が変わる。

**読んで得るもの:** 自前 LLM を OpenAI 互換にする最短距離。

## 18. Photon — Photon AI モデルの training パイプライン

PR-driven training。PR 出すと CI で train が走り、結果をコメントで返す。

**読んで得るもの:** AI training を CI に乗せる試み (うまく行った範囲)。

## 19. makimaki — 巻物的長文 reading UI

横スクロール + キーボード 操作で長文を読む実験 UI。

**読んで得るもの:** 「縦読みじゃなくていいのでは」という UX 仮説の実装。

## 20. tegata — 電子手形検証ツール

日本の商習慣 (約束手形 / 電子手形) を Web で扱う。

**読んで得るもの:** 日本特有 fintech 領域の素朴な実装。

## 21. thestandard — "the standard"

標準的なものを再定義する実験リポ。TypeScript。

**読んで得るもの:** これは正直、僕の遊び場の側面が大きい。コードより思考プロセスを読む価値。

---

## 全部読むと何が得られるか

通読すると見える共通パターンがいくつかある。

1. **Rust + Fly.io + libsql/SQLite** の繰り返し。これが今の僕の "default stack" だ。各リポで微妙にチューニングが違う。
2. **Stripe + Supabase + Fly secrets** の組み合わせ。本番投入時の地雷リストはコミット履歴の `feedback_*` ファイルに集まってる。
3. **fastlane + App Store Connect API の落とし穴** が iOS リポに繰り返し出てくる。 これは本当に毎回踏むので、 一回読んでおくと iOS 出す時に1日節約できる。
4. **AI を本番に乗せる時の安全装置** (rate limit / prompt injection 対策 / session 分離) — [昨日のブログ](/blog/2026-05-19-open-source-stop) で書いた nanobot #42 #43 の修正もこの系譜。

つまり MSA で買えるのは「個別の機能コード」ではなく **「1人 founder が 14本のプロダクトを並列に動かす時のリアルな試行錯誤」**だ。 公開できないのはセキュリティ上の理由だが、 読む価値は十分にある と自負している。

---

### 👉 [Tシャツ買って中身全部見る](https://wearmu.com/source)

¥4,900〜 · 21リポ + 将来追加 (First 100 限定) · NFT・ウォレット不要 · メールだけで照合

---

明日は **Phase 2 (1リポ end-to-end DL の実装)** に入る。trio から開ける予定だ。
