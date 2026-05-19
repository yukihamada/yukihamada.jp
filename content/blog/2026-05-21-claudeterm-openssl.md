---
title: "claudeterm — rust-openssl の CVE 7発、 1週間で 5バージョン 更新した話"
date: 2026-05-21
tags: [Rust, OSS, セキュリティ, Dependabot, rust-openssl, rust-s3, MSA, "MU Source Access"]
description: "Dependabot を有効化したら claudeterm に rust-openssl の CVE が 7発 出てきた。cargo update -p openssl で済むと思ったら、 直接依存 ではなく rust-s3 0.34 経由で rustls 0.21 が pin されていて、 結果 rust-s3 0.37 への major bump と API破壊 (Bucket::new → Box<Bucket>) を 同時 に 食う 羽目に なった。 検出 → 修正 → PR merge までの 1週間 の 試行錯誤。"
---

[MSA 中身解剖 ハブ](/blog/2026-05-20-msa-inside) の 深掘り 1本目。 [`claudeterm`](https://github.com/yukihamada/claudeterm) で 起きた、 「軽い 依存更新」 で 始まって 「lockfile 全部 揺らす」 で 終わった 1週間 の 話。

## 0. 発端

2026-05-19、 yukihamada/* の **全リポ で Dependabot を 一斉 有効化** した (理由 は [前々日 の OSS停止 blog](/blog/2026-05-19-open-source-stop) 参照)。 数分後 に GitHub から アラート の シャワー。

claudeterm に 出てきた のは **11件**:

- openssl: CVE-2026-44662, CVE-2026-42327, CVE-2026-41676, CVE-2026-41677, CVE-2026-41678, CVE-2026-41898, CVE-2026-41681 (7発)
- rustls-webpki: 3件 (DoS via malformed CRL + name constraints の 2系統)
- rand: 1件 (custom logger を 使った時 の soundness 問題)

「全部 cargo update で 済むやろ」 と 思った。 **済まなかった。**

## 1. 最初 の 仮説: cargo update -p openssl

```bash
cargo update -p openssl
```

結果:

```
note: pass `--verbose` to see 1 unchanged dependencies behind latest
```

**何も 更新 されない。** 落ち着いて `cargo tree -i openssl` で 逆引き。

```
openssl v0.10.76
└── rust-s3 v0.34.0
    └── claudeterm v0.1.0
```

直接依存 が `openssl ^0.10` だと、 `0.10.76 → 0.10.80` への 更新 は patch 範囲 なので 出来そうな はず。 でも 動か ない 理由 は すぐ判った: **rust-s3 0.34 が rustls 0.21 を pin** していて、 そっち と バージョン関係 で openssl 0.10.76 が固定されていた。

これ Rust の semver で よく 起きる。 「ある crate が 別 の crate の 旧バージョン に pin してる」 系。 解決 は **ハブ となる crate を bump する しか ない**。

## 2. 仮説 2: rust-s3 を minor bump

```bash
cargo update -p rust-s3
```

これ も 動か ない (Cargo.toml で `^0.34` 指定 して る ので)。 Cargo.toml を 開いて:

```toml
rust-s3 = "0.34"
```

を `"0.37"` に手書き 変更。 `cargo update` を 再実行。 動いた。 が…

```
error[E0308]: mismatched types
  --> src/storage.rs:91:25
   |
91 |     let bucket = s3::Bucket::new(...)?;
   |                  ^^^^^^^^^^^^^^^ expected `Box<Bucket>`, found `Bucket`
```

**API破壊**。 `rust-s3 0.37` で `Bucket::new` の 戻り値 が `Result<Box<Bucket>, _>` に変わっている。

## 3. リリースノート を 読む

[rust-s3 CHANGELOG](https://github.com/durch/rust-s3/blob/master/CHANGELOG.md) を 開く。 0.34 → 0.37 で 何 が 変わった か:

- 0.35: tokio 1.x への full migration
- 0.36: rustls 0.22 への bump
- 0.37: **`Bucket` を `Box` で 返す ように 変更** (memory layout の 都合)

3 つ の minor バージョン に 渡って 機能 が 積まれている。 全部 一度 に 食う 必要 がある。

## 4. 修正

`src/storage.rs` の 該当箇所:

```rust
// Before (0.34):
fn make_bucket() -> Result<s3::Bucket, String> {
    s3::Bucket::new(...)
}

// After (0.37):
fn make_bucket() -> Result<Box<s3::Bucket>, String> {
    s3::Bucket::new(...)
}
```

呼び出し側 は `&bucket` で 渡すケース が ほとんど で、 `Box<Bucket>` も `Bucket` も auto-deref で 同じ扱い に なる。 つまり **call site の 修正 は 不要**、 関数 シグネチャ だけ 変えれば 済んだ。

これ が rust-s3 メンテナ の 設計判断 として **ものすごく 親切** だ と あとから 気づいた。 ABI 破壊 を 最小限 に 抑える ため に Box を 使った 形 になっている。

## 5. cargo check と cargo audit を 通す

```bash
$ cargo check --workspace
warning: `dead_code` in src/email_templates.rs (pre-existing)
Finished `dev` profile in 24.5s
exit 0

$ cargo audit
No vulnerabilities found
```

11件 の Dependabot アラート が **全 close 候補** に なった。

## 6. PR と merge

[claudeterm PR #16](https://github.com/yukihamada/claudeterm/pull/16):

- `Cargo.toml`: `rust-s3 0.34 → 0.37`
- `Cargo.lock`: openssl `0.10.76 → 0.10.80`, rand `0.8.5 → 0.8.6`, rustls-webpki `0.103.10 → 0.103.13`, **古い rustls-webpki 0.101.7 が 完全に 消えた**
- `src/storage.rs`: `Box<Bucket>` への シグネチャ 更新
- `cargo check --workspace` 緑、 `cargo audit` 0件

merge 後、 GitHub Dependabot が 11件 を 自動 close。

## 7. 学んだこと

- **「軽い 依存 更新」 と 思っても まず `cargo tree -i <pkg>` で 逆引き**。 直接依存 か transitive か で 戦略 が 別物。
- **transitive 経由 で pin されている 場合、 ハブ crate の minor/major bump が 必須**。 cargo update -p の 親 を 撃ち抜く イメージ。
- **major bump の リリースノート は 必ず 全 minor を 通読 する**。 0.34→0.37 で API破壊 が 1箇所 だけ あった (Box化)、 知っていれば 5分 で 済む。
- **cargo audit は merge前 に 走らせる**。 lockfile の 揺らぎ で 別 の crate に 古いバージョン が 残る ケース が ある。

## 8. 派生 — 同じ 構造 を kagi でも 踏んだ

同じ 1週間 で `kagi` (Smart Home iOS+Rust) でも **rust-openssl 系 12件 + jsonwebtoken** を 食った。 こちら は jsonwebtoken の major (`9 → 10`) も 同時 に 食ったが、 API は 互換 (`Header`, `Algorithm::ES256`, `EncodingKey::from_ec_pem`, `encode` の シグネチャ 不変) だった ので `Cargo.toml` 1行 と `Cargo.lock` の 揺れ で 終わった。

「rust-openssl 系 を 触る なら 全リポ 同時 に やる」 が 結論。 [kagi の PR #1](https://github.com/yukihamada/kagi/pull/1) も 同じ 日 に merge した。

---

これ が claudeterm の OSS-stop 直前 1週間。 PR 単位 で コード が 全部 残っている。 [MSA](https://wearmu.com/source) で 読める。

> **👉 [Tシャツ買って 全 21 リポ を 読む](https://wearmu.com/source)** — ¥4,900〜

明日 は [pasha — fastlane の Apple ID auth に 戻した 経緯](/blog/2026-05-22-pasha-fastlane)。
