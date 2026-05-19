---
title: "pasha — fastlane の Apple ID auth に 戻した 経緯"
date: 2026-05-22
tags: [iOS, fastlane, "App Store Connect", "Apple ID", pasha, MSA, "MU Source Access"]
description: "App Store Connect API キー認証で fastlane deliver を CI 自動化 しようとして、 iOS アプリ の 審査提出 だけ API キー では できない ことを 当日 知った。 公式ドキュメント に 明記 されてない、 spaceship のソースコードを読まないと 分からない 落とし穴。 結果 1Password 経由 で Apple ID auth に 戻した話。"
---

[MSA 中身解剖 ハブ](/blog/2026-05-20-msa-inside) の 深掘り 2本目。 [`pasha`](https://github.com/yukihamada/pasha) (レシート 経費管理 iOS) で 踏んだ 「fastlane + Apple 周辺 の 落とし穴」 の 話。

## 0. 何 が 起きた か

3ヶ月 前、 pasha の 審査提出 を **GitHub Actions の CI から 自動化** しようとした。 fastlane の 公式 ドキュメント を 読んで:

> Apple recommends using App Store Connect API keys for CI/CD. They're scoped, revocable, and don't require 2FA.

「これ や」 と 思って API キー を 発行、 fastlane の Appfile に 設定、 `fastlane deliver --submit-for-review` を CI で 走らせた。

```
✗ Error: Cannot use App Store Connect API for App Review submissions
```

**は？**

## 1. ドキュメント を 読み直す

App Store Connect API の 公式 リファレンス (developer.apple.com/app-store-connect/api/) を 開く。 全 endpoint を grep:

```
GET    /v1/apps
GET    /v1/builds
POST   /v1/builds/{id}/relationships/individualTesters
POST   /v1/preReleaseVersions
... (200+ endpoints)
```

**"submission" や "review" を含む endpoint が 1つ も ない**。 App Store Connect API は 「アプリ の メタデータ 管理」 と 「TestFlight 配信」 は できるが、 **「正式 審査提出」 だけは できない**。

これ、 公式 docs の どこ にも 「審査提出 は API 経由 では 出来ません」 と 明記 されてい ない。 endpoint が 単に 「存在 しない」 だけ。 「無い こと の 証明」 を 自力 で やる ハメに なった。

## 2. fastlane の deliver ソース を 読む

fastlane の `deliver` action を grep:

```bash
$ git clone https://github.com/fastlane/fastlane
$ rg "submit_for_review" fastlane/deliver/
```

`fastlane/deliver/lib/deliver/submit_for_review.rb` の 中:

```ruby
def submit
  # Use spaceship (Apple ID auth) — no API token path
  app = Spaceship::Tunes::Application.find(@app_identifier)
  app.create_submission(...)
end
```

**spaceship 経由 で Apple ID + パスワード + 2FA で 認証 する path しか ない**。 これ で 確信。

ちなみに spaceship は Apple の **非公開 internal API** を リバース エンジニアリング した ライブラリで、 Apple ID 認証 を ブラウザ から の リクエスト として 偽装 して 通す。 ABI も 仕様 も 約束 されて ない。 突然 動か なくなる ことが あり、 過去 何度も Apple が UI を 変えるたび に fastlane コミュニティ が 大慌て で 直してきた 経緯 が ある。

## 3. 1Password 経由 で Apple ID auth に 戻す

CI で Apple ID 認証 を 通す には 2FA を 自動化 する 必要 が ある。 オプション:

1. **`spaceauth` で セッション cookie を 取得 して 環境変数 に 入れる** — セッション が 数日 で 切れる ので CI が 突然 落ちる
2. **専用 の Apple ID を 作って 2FA を 信頼デバイス で 受ける** — でも GitHub Actions の runner に SMS を 紐付けられ ない
3. **1Password Service Account の SSH/CLI 経由 で 2FA code を 取り出す** — fastlane に `FASTLANE_SESSION` を 注入

採用 した の は (3)。 ローカル の Mac で:

```bash
$ op signin
$ op item get "Apple ID — pasha" --otp
123456
```

を `fastlane spaceauth -u $APPLE_ID` の 流れで 取得、 結果 を GitHub Actions の secret に 投入。 セッション は 30日 持つ ので、 月初 に 手動 更新。

## 4. fastlane の Appfile

```ruby
# Before (API key approach — failed)
app_store_connect_api_key(
  key_id: ENV['ASC_KEY_ID'],
  issuer_id: ENV['ASC_ISSUER_ID'],
  key_filepath: 'AuthKey.p8'
)

# After (Apple ID approach — works)
apple_id ENV['APPLE_ID']
team_id  ENV['TEAM_ID']
# FASTLANE_SESSION is read from env automatically
```

これで `fastlane deliver --submit-for-review` が GitHub Actions から 通る ように なった。

## 5. 副産物 — 審査リジェクト の 理由 を 取る も spaceship 経由

審査 が rejected された 場合、 拒否 理由 を プログラム から 取り出したい こと が ある。 こちら も **App Store Connect API には 取り出す endpoint が ない**。 spaceship 経由 で 取る しか ない:

```ruby
require 'spaceship'
Spaceship::ConnectAPI.login(ENV['APPLE_ID'], ENV['APPLE_PASSWORD'])

app = Spaceship::ConnectAPI::App.find('com.enabler.pasha')
sub = app.get_app_store_review_submissions(filter: { state: 'IN_REVIEW' }).first
puts sub.review_notes  # ← ここに human-readable な reject 理由
```

これ 知らなかった ら 「リジェクト 来た → App Store Connect の Web UI 開く → 拒否 通知 を copy paste → 修正」 という 手動 ループ から 抜けられない。

## 6. Pasha の fastlane Gemfile.lock の Dependabot 騒動 (続き)

先週 ([2026-05-19](/blog/2026-05-19-open-source-stop)) の OSS-stop の 流れ で、 pasha の Gemfile.lock にも Dependabot 警告 が 4件 出てきた:

- jwt: CVE-2026-45363 (空鍵 HMAC バイパス)
- addressable: CVE-2026-35611 (ReDoS)
- json: CVE-2026-33210 (format-string injection)
- faraday: CVE-2026-25765 (SSRF)

`bundle update jwt addressable json faraday --conservative` で 3件 は patch できた。 でも **jwt だけ patch できなかった**。 理由 が 笑える: **fastlane 2.234.0 (最新版) が `jwt < 3` を pin している**。 jwt 3.x が CVE-fix版 だが、 fastlane が 来てくれない 限り バンプ できない。

[PR #1](https://github.com/yukihamada/pasha/pull/1) は 3件 fix + jwt は **upstream の fastlane が pin を 緩める まで 保留** と 書いて merge した。 release tooling 用 の Ruby bundle なので 攻撃面 は 限定的、 という 判断。

## 7. 学んだこと

- **「公式 が API を 用意 してる = 全機能 が 使える」 は 誤り**。 endpoint が 単に 存在 しない 操作 (App Store の submit_for_review) が ある。 docs に 「無い」 と 書いて ない だけ。
- **不足 を 知る には 「使われて いる 既存 ライブラリ の ソース を 読む」 のが 早い**。 fastlane の deliver/spaceship を 読まなかった ら 半日 ハマって いた。
- **Apple の internal API を リバース した spaceship に 頼る しか ない 操作 が 存在 する** — submit_for_review、 reject 理由 取得、 IAP プロモコード 一括発行 etc。 公式 API では カバー されない。
- **依存 update の patch 漏れ は 理由 を 明記** して merge する。 「全部 fix」 を 装う より 「3/4 fix、 1 は 待ち」 と 書く ほうが 健全。

これ 全部、 fastlane Apple Dev 界隈 では 「皆 が 知っていて 誰も documented してい ない」 暗黙知 と 化して いる。 pasha の CI ファイル と Gemfile を 読むと、 ハマった 順序 が commit log で 追える。

---

> **👉 [Tシャツ買って pasha + 残り 20 リポ を 読む](https://wearmu.com/source)** — ¥4,900〜

明日 は [nanobot — 公開 issue として 放置した prompt injection と CORS reflect、 直すまで](/blog/2026-05-23-nanobot-security)。
