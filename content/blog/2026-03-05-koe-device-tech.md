---
title: "Koe Device技術仕様 — ESP32-S3 Rust + P2P UDP + 技適対応"
date: "2026-03-05"
description: "Koe Deviceのハードウェア設計、ESP32-S3 Rustファームウェア、P2P UDPプロトコル、技適対応までの技術仕様書。"
tags: ["Koe Device", "ESP32", "Rust", "tech"]
---

## Koe Deviceとは

Koe（声）は音声入力デバイスだ。マイクで拾った音声をリアルタイムにテキスト変換し、P2P UDPでLAN内のデバイスに配信する。クラウドを経由しないため、レイテンシは50ms以下、プライバシーリスクはゼロ。BOM $24で量産可能なCOIN（Compact Open Input Node）設計を採用している。

## ハードウェア仕様

### MCU: ESP32-S3-MINI-1

ESP32-S3-MINI-1を選定した理由は3つ。技適認証済み（201-220017）、Xtensa LX7デュアルコア（240MHz）でオンデバイス推論に十分な性能、そしてUSB OTGによるPCB簡略化だ。

```
+--------------------------------------------------+
|                 Koe COIN v1.0                     |
|                                                   |
|  [USB-C]---+     +--[ESP32-S3-MINI-1]--+         |
|            |     |                       |         |
|     VBUS---+--[LDO 3.3V]---VDD         |         |
|     D+ --------GPIO19 (USB_D+)         |         |
|     D- --------GPIO20 (USB_D-)         |         |
|     GND--------GND                      |         |
|                  |                       |         |
|         GPIO4 --+-- I2S_BCK             |         |
|         GPIO5 --+-- I2S_WS              |         |
|         GPIO6 --+-- I2S_DIN             |         |
|                  |                       |         |
|         GPIO2 --+-- NeoPixel (WS2812B)  |         |
|         GPIO0 --+-- BOOT button         |         |
|         EN -----+-- Reset button        |         |
|                  +----------------------+         |
|                                                   |
|  [INMP441]     [WS2812B x1]     [Buttons x2]     |
|  MEMSマイク     状態LED           BOOT/RST        |
+--------------------------------------------------+

BOM (100台ロット):
  ESP32-S3-MINI-1     $3.20
  INMP441 MEMSマイク   $1.80
  USB-C コネクタ       $0.30
  LDO (AMS1117-3.3)   $0.15
  WS2812B LED          $0.10
  受動部品 + PCB       $4.50
  ケース (射出成型)    $3.50
  組立 + 検査          $10.45
  ───────────────────────────
  合計                 $24.00
```

### I2Sマイク (INMP441)

INMP441はI2Sインターフェースの底面ポートMEMSマイクで、SNR 61dB、感度 -26dBFS。I2Sはアナログ変換を介さないためノイズに強く、ADCを省略できるのでBOMと回路を削減できる。サンプリングレートは16kHz/16bitで、音声認識モデルの入力に合わせている。

## Rustファームウェア

`esp-idf-hal`と`esp-idf-svc`を使い、Rust (std環境) でファームウェアを開発する。no_stdではなくstdを選択した理由は、WiFiスタックとTCP/UDPソケットAPIの利用にstd環境が必要なためだ。

```rust
// firmware/src/main.rs
use esp_idf_hal::i2s::{I2sDriver, I2sRx, config::*};
use esp_idf_svc::wifi::EspWifi;
use std::net::UdpSocket;

const MULTICAST_ADDR: &str = "239.42.42.1";
const MULTICAST_PORT: u16 = 4242;
const SAMPLE_RATE: u32 = 16_000;
const CHUNK_MS: u32 = 20;  // 20msチャンク = 640サンプル = 1280バイト
const CHUNK_SAMPLES: usize = (SAMPLE_RATE * CHUNK_MS / 1000) as usize;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    // WiFi接続（WPA2-PSK、NVSからSSID/PASS読み込み）
    let mut wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?;
    wifi_connect(&mut wifi)?;

    // NTP同期（P2Pタイムスタンプ用）
    sntp_sync()?;

    // I2S初期化（INMP441 MEMSマイク）
    let i2s_config = I2sConfig::new()
        .sample_rate(SAMPLE_RATE)
        .bits_per_sample(BitsPerSample::Bits16)
        .channel_format(ChannelFormat::OnlyLeft)
        .communication_format(CommunicationFormat::I2sStandard);
    let i2s = I2sDriver::new_rx(
        peripherals.i2s0,
        &i2s_config,
        peripherals.pins.gpio4,  // BCK
        peripherals.pins.gpio5,  // WS
        Some(peripherals.pins.gpio6),  // DIN
    )?;

    // UDPマルチキャスト送信
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let dest = format!("{}:{}", MULTICAST_ADDR, MULTICAST_PORT);

    let mut buf = vec![0i16; CHUNK_SAMPLES];
    loop {
        i2s.read(&mut buf)?;
        let packet = KoePacket::new(&buf);
        socket.send_to(&packet.serialize(), &dest)?;
    }
}
```

## P2P UDPパケットフォーマット

Soluna P2Pプロトコルに準拠したUDPマルチキャスト（239.42.42.1:4242）でオーディオチャンクを配信する。パケット構造は以下の通り。

```
Koe Audio Packet (1296 bytes):
+--------+--------+--------+--------+
|  Magic (4B): "KOE\x01"           |  バージョン1識別子
+--------+--------+--------+--------+
|  Device ID (6B): MAC address      |  送信元デバイス識別
+--------+--------+--------+--------+
|  Sequence (4B): u32 big-endian    |  欠損検知用シーケンス番号
+--------+--------+--------+--------+
|  Timestamp (8B): u64 microsec     |  NTP同期済みUnixタイムスタンプ
+--------+--------+--------+--------+
|  Audio Data (1280B):              |  16kHz/16bit/mono = 640サンプル
|  20msチャンク、リトルエンディアン   |  = 20ms分のPCMデータ
|  ...                              |
+--------+--------+--------+--------+

ヘッダ: 16 bytes + ペイロード: 1,280 bytes = 合計 1,296 bytes
MTU 1500以下に収まるためフラグメンテーション不要
```

受信側は`Sequence`番号でパケットロスを検知し、前後のサンプルで線形補間する。LAN内であればパケットロス率は0.01%未満だが、WiFi環境では1-2%発生するため補間処理は必須だ。

## NTP同期

P2Pで複数デバイスの音声を合成する場合、タイムスタンプの精度が重要になる。ESP-IDFの`esp_sntp`を使い、起動時にNTPサーバー（`ntp.nict.jp`）と同期する。同期精度は10ms以内で、音声のリップシンクには十分だ。

再同期は1時間ごとに実行し、RTC（ESP32-S3内蔵）のドリフトを補正する。WiFi切断中はRTCのみで計時し、再接続時にNTP再同期する。

## 技適対応

ESP32-S3-MINI-1は技適認証番号201-220017を取得済みのため、モジュールをそのまま使えば追加の技適申請は不要だ。ただし以下の条件を満たす必要がある。

1. **アンテナ**: モジュール内蔵PCBアンテナのみ使用。外部アンテナを接続する場合は再認証が必要
2. **送信出力**: ファームウェアでWiFi送信出力を変更しない（デフォルト: 20dBm）
3. **表示義務**: 筐体に技適マークと認証番号を表示（レーザー刻印 or ラベル）

## 消費電力と電源設計

| モード | 消費電流 | 持続時間 (1000mAh) |
|--------|---------|-------------------|
| アクティブ（WiFi + I2S + UDP送信） | 160mA | 6.25時間 |
| Light Sleep（WiFi維持、マイクOFF） | 5mA | 200時間 |
| Deep Sleep | 10uA | 11,400時間 |

USB-C給電を前提としているが、モバイル用途ではLiPoバッテリーを追加可能。BQ24075充電ICを追加した場合、BOMは$27.50になる。

## まとめ

Koe DeviceはBOM $24、技適対応済み、クラウド不要のP2P音声入力デバイスだ。ESP32-S3のRust開発環境は成熟しつつあり、std環境であればPCアプリケーションに近い開発体験が得られる。UDPマルチキャストによるゼロコンフィグ配信は、LAN内IoTの設計パターンとして応用範囲が広い。
