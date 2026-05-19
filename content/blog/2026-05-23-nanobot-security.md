---
title: "nanobot — 公開 issue として 放置 した prompt injection と CORS reflect、 直すまで"
date: 2026-05-23
tags: [nanobot, chatweb.ai, セキュリティ, "prompt injection", CORS, Rust, MSA, "MU Source Access"]
description: "chatweb.ai のバックエンド nanobot で、 /api/v1/chat の session_id を 全リクエスト で共有して別ユーザーの会話に prompt injection できる状態だった (#43)。 同時にCORS が任意の Origin を反射していた (#42)。 自分で issue として 公開放置 した結果、 攻撃側に 看板 を立てる状態に なっていた。 検出→修正の全過程。"
---

[MSA 中身解剖 ハブ](/blog/2026-05-20-msa-inside) の 深掘り 3本目、 最終回。 [`nanobot`](https://github.com/yukihamada/nanobot) で 起きた 2件 の 実バグ を、 公開 で 放置 して しまっていた 話。 一番 きつい。

## 0. 発端

nanobot は [chatweb.ai / teai.io](https://teai.io) の バックエンド。 Rust + Axum + Lambda → 今は Fly.io の `chatweb-ai` で 動いて いる。 47 件 の open issue が ある。

そのうち 2件 が **僕 自身 が セキュリティ bug として 立てた もの**:

- **#43** — All API requests share `api:default` session — no user isolation
- **#42** — CORS reflects arbitrary `Origin`

両方 とも 2026-03-上旬 に 自分 で 検出 して issue を 切った。 「直そう」 と 思って いた。 **2ヶ月 半 放置 していた**。

## 1. #43 — 同じ session_id を 全リクエスト で 共有

### 症状

chatweb.ai の `POST /api/v1/chat` を カール:

```bash
curl -X POST https://chatweb.ai/api/v1/chat \
  -d '{"message": "前回の話の続きですが"}'
```

返って くる 応答 が **誰か 別 の 人 の 会話 を 参照** している ような 動き を 見せた。 「あれ、 これ session どうやって 分けて るんだっけ」。

ソース を 読む。 `crates/nanobot-core/src/service/http.rs`:

```rust
fn default_session_id() -> String {
    "api:default".to_string()
}
```

**全リクエスト が 同じ ID** を 使って いた。 結果:

- ユーザー A が KV store に 「私の API キー は sk-xxx」 と 書く
- ユーザー B が 「KV から 取り出して」 と 投げる
- nanobot は session "api:default" の KV を 読む
- **B に A の API キー が 返される**

これ が cross-request prompt injection の 確認 された 形 だった。 「会話 履歴」 の 共有 だけ なら まだしも、 ツール呼び出し で メモリ や ファイル に アクセス できる ので、 影響範囲 は 「攻撃者 が 他人 の メモリ に 任意 の prompt を 残す」 まで 広がる。

### 修正

`default_session_id` を request 毎 に UUID v4 を 返すように 変更:

```rust
use uuid::Uuid;
fn default_session_id() -> String {
    format!("api:{}", Uuid::new_v4())
}
```

クライアント が 明示的 に `session_id` を 渡す ケース は 既存挙動 と 同じ。 渡さない 場合 に **「全部 共有」 ではなく 「全部 isolation」** にした。

既存 の `validate_session_id()` の regex が `^[a-zA-Z0-9_:-]+$` を 受け付ける ので UUID-v4 形式 (ハイフン入り) も そのまま 通る。 source code 変更 は 25行 削除 / 134行 追加 (= regex 強化 と test 4本 追加 を 含む)。

## 2. #42 — CORS reflect

### 症状

```bash
$ curl -i -H "Origin: https://attacker.example.com" https://chatweb.ai/api/v1/chat
HTTP/1.1 200 OK
Access-Control-Allow-Origin: https://attacker.example.com  ← 反射！
Access-Control-Allow-Credentials: true
```

任意 の Origin を 反射 して、 さらに `Allow-Credentials: true` を 返して いた。 ブラウザ から の cross-origin 認証付き リクエスト を 通せる 構造。

### 経緯

実 は #42 は **既に 直して いた**。 commit `34ad706` (2026-02-08) で `AllowOrigin::list(...)` に 変更済み。 でも issue は close してい なかった。

なぜ か? 当時 の `AllowOrigin::list` の 中 に **`std::env::var("BASE_URL").ok()`** が 入って いて、 これ が ランタイム で 任意 の Origin を 許可 する 抜け穴 に なって いた。 つまり 「修正 した つもり が 抜け穴 を 残して いた」。

### 修正 (今回)

3つ 変更:

1. `cors_allowed_origins()` という 専用関数 を 切り出す。 BASE_URL は **完全 削除**。
2. compile-time gate で `cfg(not(debug_assertions))` の 時 だけ release origin (chatweb.ai / teai.io) に 絞る。 dev 時 は localhost も 通る。
3. **「将来 mirror_request() / predicate-true / Origin reflection に絶対 切り替える な」** の コメント を 強い 言葉で 残す。

test を 3本 足した:

- `test_cors_allowlist_contains_production_origins`
- `test_cors_allowlist_rejects_arbitrary_origin`
- `test_cors_allowlist_release_excludes_localhost` (`#[cfg(not(debug_assertions))]`)

これ で 「未来 の 自分 が 同じ 抜け穴 を 開け 直す」 を blocking した。

## 3. PR は 1本 で 出した

[nanobot PR #83](https://github.com/yukihamada/nanobot/pull/83):

- `crates/nanobot-core/src/service/http.rs` — +134/-25
- Closes #42, #43

```
$ cargo check -p nanobot-core
warning: 16 pre-existing unused-var
Finished `dev` profile in 2.65s
exit 0

$ cargo check -p nanobot-core --features fly
exit 0
```

`cargo test --features fly` は 走らなかった (9件 の 別 件 で 既存 の test code が compile エラー、 main にも 同じ エラー あり)。 PR body に 明記 して merge。

## 4. 一番 きつかった こと — issue を 公開 で 放置 して いた

技術的 な 修正 は 200行 で 終わる。 きつかった の は **2ヶ月半 もの あいだ 「ここに 穴 が ある」 と 公開 で 看板 を 立てて いた** こと。

僕 が 立てた issue タイトル は こうだった:

- `[Bug] All API requests share api:default session — no user isolation`
- `[Security] CORS: Access-Control-Allow-Origin reflects arbitrary origins`

**書いて ない 攻撃手順 を 想像 する 攻撃側 にとって 「reproducer 1割 + ヒント 8割 + 修正 まだ 1割」 の 状態**。 これ が public repo の issue 欄 に **2ヶ月半** 載って いた。

triage の 段階 で 「公開 issue では なく Security Advisories の private 報告 経路 を 使え」 と 自分 で 自分 に 言って いれば、 こうは ならなかった。 でも 1人 で 全部 書いてる と、 「自分 への TODO」 と 「公開 への 通知」 が 同じ 場所 (issue tracker) に 混ざる。 これ が 構造的 に 危ない。

OSS停止 ([2026-05-19 の blog](/blog/2026-05-19-open-source-stop)) の トリガー の 一つ は、 まさに この 「自分 で 公開 で 穴 を 晒した」 経験。

## 5. 学んだこと

- **「将来 直す つもり」 の bug は 必ず private で 管理 しろ**。 GitHub Security Advisories の **draft** か、 自分 の private リポ の issue。 public 直書き は 絶対 NG。
- **「修正 した つもり」 の commit に は test を 残せ**。 #42 の `BASE_URL` 抜け穴 は test が なかった ので 半年 気付か なかった。
- **Default 値 は 「全部 独立」 を 選べ**。 「全部 共有」 を default に した 設計判断 は 速度 を 取って 安全 を 落として いた。
- **公開 issue の triage は ML 並 に 優先度 を 上げる**。 1ヶ月 以内 に 必ず 「fix or move to private」 を 決める。

## 6. 副産物 — 47 → 3 issues

nanobot の 47 件 の open issue を triage したら、 **45件 が dog-pack agent (自前 の AI agent fleet) が 自動 で 立てた spam** だった。 重複・obsolete・out-of-scope を bulk close で 41件 + dup 1件 を close、 残った 5件 のうち #42 #43 を merge で auto-close。 最終 残り **3件** (CONTRIBUTING / rate-limit / cargo-audit、 全て 実 タスク)。

dog-pack 側 は `rustydog/spin-component/src/heartbeat.rs:699` の `attempt_cross_project` 関数 が auto-file を 担当 して いて、 **per-repo の cooldown が 無かった** ので 同じ issue を 何度 も 立てて いた。 18行 の patch で `7日 cooldown + 最新 dog-pack issue に コメント無し なら 24h backoff` を 入れる proposal を 書いた (`/tmp/dog-pack-rate-limit-proposal.md`)。 30 instance に rollout する のは 来週。

---

これ で MSA 深掘り 3本 完了。 残り 18 リポ は [ハブ](/blog/2026-05-20-msa-inside) の 一覧表 を 見て、 興味 ある の が あれば MSA で 中身 を 読んで ほしい。

> **👉 [Tシャツ買って 21 リポ 全部 読む](https://wearmu.com/source)** — ¥4,900〜 · 初の 5名 招待 通し テスト は 2026-05-31。

[← OSS停止 の理由 へ戻る](/blog/2026-05-19-open-source-stop)
