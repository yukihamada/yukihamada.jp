---
title: "SOLUNA Protocol — 30億人が同じビートを同じ瞬間に感じる技術"
date: 2026-03-18
tags: [SOLUNA, Protocol, Audio, P2P, Sync]
description: "フェスで30億台のデバイスが同時に鳴る。RTP互換の24バイトOSTPヘッダで、音速の壁を超える同期を実現する。"
---

# 30億人が同じビートを同じ瞬間に感じる

フェスに行くと、ステージから離れるほど音がズレる。音は秒速343mでしか進まない。50m離れたら145msの遅延。人間が「同時」と感じる限界は5ms。

**もし会場の全員のデバイスから、同じ瞬間に同じ音が出たら？**

これがSOLUNA（Open Sonic Transport Protocol）で実現したことだ。

---

## ヘッダ構造: RTP互換 24バイト

WebRTCやTCPの分厚いスタックを全部剥がして、UDP + RTP互換の薄いヘッダだけで音声を送る。

```
Offset  Size  Field
──────  ────  ─────
[RTP Header — 12 bytes (RFC 3550互換)]
0       1B    V=2, P, X=1, CC
1       1B    M, PT (96=PCM, 116=ADPCM)
2-3     2B    Sequence Number
4-7     4B    RTP Timestamp (48kHz clock)
8-11    4B    SSRC (送信元ID)

[Extension Header — 4 bytes]
12-13   2B    Profile: 0x4F53 ("OS")
14-15   2B    Length: 2 (32-bit words)

[OSTP Header — 8 bytes]
16-17   2B    Stream ID (上位4bit=チャンネル数)
18-19   2B    Sequence Extension (32bit拡張)
20-23   4B    Media Timestamp (wall-clock ms)

[Payload]
24+     nB    Audio (max ~400B, MTU 1500内)

[Trailer]
last 4B       CRC-32 (整合性検証)
```

なぜRTP互換か: AES67、Dante、既存の放送機器とゲートウェイで繋がる。独自プロトコルの孤島にならない。

パケットサイズ: 96サンプル × 4バイト = 384バイト + ヘッダ24バイト = **408バイト**。MTU 1500の1/3以下。フラグメンテーション一切なし。

---

## Raw First Strategy — 生音を先に送る

SOLUNAの核心技術。IMA-ADPCMの「初期値問題」を逆手に取るハック。

```
Time 0ms:   PT=96 (生PCM) × 5パケット → DMAに直接投入 → 遅延ゼロ再生
            最後のサンプル → ADPCMのvalprev (初期値) にセット

Time 10ms:  PT=116 (ADPCM) に自動切替 → 帯域75%削減
            デコーダはseeded valprevから完璧にデコード → ノイズなし
```

**なぜ天才的か:** ADPCMは最初の1サンプルの基準値がないと正確にデコードできない。最初に生PCMを送ることで、その問題を完全に解消しつつ、即座に帯域を1/4に落とせる。

---

## 同期処理 — 地球の裏側のデバイスとピッタリ合わせる

### Step 1: NTPクロック同期

各デバイスがリレーサーバーと4点タイムスタンプ交換:

```
デバイス → [T1] → リレー
リレー → [T2, T3] → デバイス → [T4]

offset = ((T2-T1) + (T3-T4)) / 2
```

適応EMA: 最初50回はα=0.20で素早く収束、安定後はα=0.02でゆっくり追従。

### Step 2: MAXDELAY — 全員を最も遅いデバイスに合わせる

```
Mac (50ms遅延) + iPhone (60ms遅延) → リレー: MAXDELAY=160ms
全デバイスが160msバッファ → 同じ瞬間に再生
```

### Step 3: 音響チャープキャリブレーション（§17, フェス向け）

GPSは3-10m誤差 = 9-29msのズレ。プロ音響には不十分。

**解決:** ライブ前にステージPAから測定音（チャープ信号）を1発。1000台のMEMSマイクが到達時刻を計測。

```
Stage PA: チャープ発射 at T=0 (NTP同期)
Device_A: マイク検出 at T=145ms → 距離49.7m → delay=145ms
Device_B: マイク検出 at T=291ms → 距離99.8m → delay=291ms
```

気温、風向き、群衆密度、建物反射 — **全部マイクが実測**。GPS不要。サブミリ秒精度。

---

## 30億人対応 — P2Pスワームツリー

```
DJ → Origin Relay (1台)
  → Region Relays (~20台)
    → Edge Relays (~10,000台)
      → P2P Swarm Tree (K=4, depth=16)
         → 4^16 = 42億ノード
```

50人超でスワーム自動起動。各デバイスが最大4台に転送。サーバー負荷一定、リスナーは指数関数的に増加。

---

## ESP-NOW ハイブリッドメッシュ（§16, 自己修復型）

フェスの死角対策。Wi-Fiが届かない場所でも音が鳴る。

```
正常: AP → Wi-Fi → 全デバイス
死角: AP → Wi-Fi → Device_A → ESP-NOW → Device_B (死角内)
```

Wi-Fiを受信できたデバイスが、ESP-NOW（2.4GHz P2P、AP不要、遅延<1ms）で周囲に再ブロードキャスト。ルーターに依存しない自己修復型メッシュ。

---

## 周波数分割マルチキャスト（§18, 帯域最適化）

26mmのCOINスピーカーで100Hz以下の低音は物理的に鳴らない。送るだけ無駄。

```
Source → Crossover (200Hz)
  ├→ Low (<200Hz)  → 239.69.0.1 → サブウーファー
  └→ High (>200Hz) → 239.69.0.2 → COIN/スマホ
```

COIN: 帯域35%削減、バッテリー50%延長。補聴器モード（300-4kHz）なら70%削減。

---

## 著作権 — 流した瞬間に権利者に還元

30秒ごとに音声指紋 → AcoustIDで楽曲同定 → 自動分配:

```
権利者: 70% | DJ: 20% (キャッシュバック) | プラットフォーム: 10%
```

DTLS-SRTPで音声暗号化。Stripeでウォレット決済。Solanaでチップ。

---

## これで何が変わるか

| 場面 | Before | After (SOLUNA) |
|------|--------|----------------|
| **フェス** | $30万のPA機材 | $24のCOIN × 1万個 + 観客のスマホ |
| **家** | AirPlay 1台ずつ設定 | 1台再生 → 全デバイス自動同期 |
| **カフェ** | 有線スピーカー+月額BGM | `curl install-rx.sh` でラズパイ即BGM |
| **通話** | 電話番号交換 | @mention でプッシュ通知 → 即通話 |
| **配信** | YouTube/Twitch (3秒遅延) | SOLUNA (<5ms、双方向) |

---

## オープンソース (MIT License)

- **仕様**: [OSTP-SPEC.md](https://github.com/yukihamada/opensonic) (v0.9.4)
- **コード**: [github.com/yukihamada/opensonic](https://github.com/yukihamada/opensonic)
- **Mac App**: [GitHub Releases](https://github.com/yukihamada/opensonic/releases)
- **iOS**: [TestFlight](https://testflight.apple.com/join/PYbefDSE)
- **ラズパイ**: `curl -fsSL https://solun.art/install-rx.sh | sudo bash`

フェスでのパイロット・コラボ: [mail@yukihamada.jp](mailto:mail@yukihamada.jp)
