---
title: "MU Source Access の中身 — 21リポで何が読めるか (深掘り3本のハブ)"
date: 2026-05-20
tags: [MU, "MU Source Access", MSA, wearmu, OSS]
description: "MSA の 21リポ で 一番 読む 価値 が あるのは どれか。 1段落 ずつ 全部 紹介 する代わりに、 ハマって 抜けた話 3本 を 深掘り 別記事 に書く。 ここは その目次。"
---

[昨日](/blog/2026-05-19-open-source-stop)、 **MU Source Access**（MSA）を発表した。 wearmu の Tシャツ 1枚 で yukihamada/* の private リポ 21本 が 全部 読める、 という仕組み。

> **👉 [先に /source ページ を 見る](https://wearmu.com/source)**

「で、 中身 は 何 なの？」 と 聞かれた。 これに 答える 形を 1日 考え 直した。

**最初 は 21リポ を 1段落 ずつ 紹介 する 案だった**。 でも 書いたら 「読み流される 21段落」 に しかならなかった。 ターゲット 田中さん (年商1.8億 B2B SaaS創業者、 紫帯、 JiuFlow Pro 2年目) に 「説明書 になってる、 買う 気 にならない」 と 言われた。

その通りだ。

代わりに **「実際 ハマって 抜けた話」 を 3本** だけ 深く 書くことにする。 ここ は その 目次。

## 深掘り 3本

それぞれ 1リポ から 1テーマ を 取り出して、 **症状 → 仮説 が 外れた 順序 → 着地** を 書く。

### 1. [claudeterm — rust-openssl の CVE 7発、 1週間で 5バージョン 更新した話](/blog/2026-05-21-claudeterm-openssl) (2026-05-21)

`cargo audit` で 急に 出てきた openssl の CVE 7発。 `cargo update -p openssl` で済む と 思った ら、 直接 依存 ではなく `rust-s3 0.34` 経由 で `rustls 0.21` が pin されていた。 結果 `rust-s3 0.34→0.37` の API破壊 (`Bucket::new` の 戻り値 が `Box<Bucket>` 化) も 一緒に 食う ハメに なった。 [PR #16](https://github.com/yukihamada/claudeterm/pull/16) を 出した 時の 試行錯誤。

### 2. [pasha — fastlane の Apple ID auth に 戻した 経緯](/blog/2026-05-22-pasha-fastlane) (2026-05-22)

App Store Connect API キー 認証 で `fastlane deliver` を 自動化 しようとして、 **iOS アプリ の 審査提出 だけ API キー では できない** ことを 当日 知った。 結果 1Password 経由 で Apple ID auth に 戻した。 公式ドキュメント に 明記 されてない、 spaceship のソースコード を 読まないと わからない 落とし穴。

### 3. [nanobot — 公開issue として 放置 した prompt injection と CORS reflect、 直すまで](/blog/2026-05-23-nanobot-security) (2026-05-23)

`/api/v1/chat` の session_id を **全リクエスト で `"api:default"` を 共有** していて、 別ユーザー の 会話 に prompt injection ができる 状態だった (#43)。 同時に CORS が 任意の Origin を 反射 していた (#42)。 自分で issue として 放置 した結果、 公開 issue が **攻撃側に 看板 を 立てる** 状態に なっていた話。 検出 → 直すまで の 全過程。

## 残り 18 リポ は 一行 で

それ以外 の 18本 は **何屋 か** だけ 並べる。 詳細 は MSA 入って から ソース で 読むほうが 圧倒的 に 早い。

| repo | 何屋 |
|---|---|
| trio | macOS の AI メッセージ 要約 + 3カード 返信 提案 |
| kagi | スマートホーム iOS / Mac Catalyst (Hue/SwitchBot) |
| jitsuflow | 柔術 試合 フロー 記録 (Dart/Flutter + Cloudflare Worker) |
| tsugi | 継承 protocol 参考実装 (append-only + content-addressed) |
| hato | 軽量 messaging 実験 (Axum + libsql) |
| hypernews | news.xyz バックエンド (HN ライク + 半減期) |
| pon | 電子契約 iOS (電子帳簿保存法対応) |
| NOU | ローカル LLM サーバ + GUI installer (mac/win/linux) |
| factlens | URL/claim を 複数ソース cross-check |
| gitnote | git-backed note (全編集 が commit) |
| flow-anime | アニメ 続き 観たい 瞬間 用 軽量 index |
| phishguard | フィッシング訓練 SaaS MVP (Next.js + Stripe) |
| security-scanner | 中小企業 Web 診断 SaaS MVP |
| security-education | サイバーセキュリティ 教育 platform |
| nemotron | Nvidia Nemotron Nano 9B v2 を Modal/RunPod 1コマンド deploy |
| Photon | AI モデル の training を PR-driven にする実験 |
| makimaki | 横スクロール 巻物 reading UI |
| tegata | 電子手形 (約束手形) 検証ツール |
| thestandard | UI primitive を 再定義 する 実験 |

## 通読 すると 見える もの

21本 全部 読むと、 共通パターン が 浮き上がる。

- **Rust + Fly.io + libsql/SQLite** が 僕 の default stack。 各リポ で 微妙 に チューニング が 違う。
- **Stripe + Supabase + Fly secrets** の地雷 は `feedback_*.md` ファイル に 集約 してる。
- **fastlane + ASC API の 落とし穴** は iOS リポ で 繰り返し 出てくる (pasha の 深掘り 参照)。
- **AI を 本番 に 乗せる 時 の 安全装置** (rate limit / prompt injection / session 分離) — nanobot の 深掘り 参照。

つまり **1人 founder が 14本 並列 で 動かす 時 の リアル な 試行錯誤** が 買える。 公開 で 配るには 重すぎる (脆弱性 や 過去 ハードコード の リスク) が、 MSA 経由 なら 出せる。

---

### 👉 [Tシャツ買って 中身 全部 見る](https://wearmu.com/source)

¥4,900〜 · 21リポ + 将来追加 (First 100 限定) · NFT・ウォレット不要 · メール だけ で 照合

---

明日 (2026-05-21) は [**claudeterm の openssl saga**](/blog/2026-05-21-claudeterm-openssl)。
