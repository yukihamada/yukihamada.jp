---
title: "Koe技術スタック — CoreML + whisper.cpp + llama.cpp + Metal"
date: "2026-03-10"
description: "macOS/iOS/Windows対応のローカル音声入力アシスタントKoeのクロスプラットフォーム技術設計"
tags: ["Koe", "Swift", "Rust", "tech"]
---

## Koeとは

Koeはローカル完結の音声入力アシスタントだ。音声認識からテキスト補正まですべてデバイス上で処理し、外部サーバーに音声データを送信しない。macOS、iOS、Windowsの3プラットフォームに対応する。

本記事ではクロスプラットフォーム対応の技術的判断と、各プラットフォーム固有の実装詳細を解説する。

## アーキテクチャ全体像

```
+------------------------------------------------------------------+
|                        Koe Architecture                           |
+------------------------------------------------------------------+
|                                                                    |
|  macOS (Swift)              Windows (Rust)        iOS (Swift)     |
|  +-----------------+        +----------------+    +--------------+|
|  | Carbon Hot Key  |        | Win32 Hot Key  |    | Shortcut     ||
|  | (Accessibility  |        | (RegisterHotKey|    | Integration  ||
|  |  権限不要)       |        |  API)          |    |              ||
|  +--------+--------+        +-------+--------+    +------+-------+|
|           |                         |                     |        |
|  +--------v--------+        +-------v--------+    +------v-------+|
|  | AVAudioEngine   |        | WASAPI Capture |    | AVAudioEngine||
|  | 16kHz mono      |        | 16kHz mono     |    | 16kHz mono   ||
|  +--------+--------+        +-------+--------+    +------+-------+|
|           |                         |                     |        |
|  +--------v--------+        +-------v--------+    +------v-------+|
|  | CoreML Whisper  |        | whisper.cpp    |    | CoreML       ||
|  | (Large V3)      |        | + CUDA/Vulkan  |    | Whisper      ||
|  +--------+--------+        +-------+--------+    +------+-------+|
|           |                         |                     |        |
|  +--------v--------+        +-------v--------+    +------v-------+|
|  | CoreML LLM      |        | llama.cpp      |    | Soluna P2P   ||
|  | (テキスト補正)    |        | + CUDA/Vulkan  |    | (分散推論)    ||
|  +--------+--------+        +-------+--------+    +------+-------+|
|           |                         |                     |        |
|           v                         v                     v        |
|     [クリップボード / 直接入力]  [クリップボード]    [クリップボード] |
|                                                                    |
+------------------------------------------------------------------+
```

## macOS: Carbon Hot Key API の選択

macOSのグローバルホットキーには3つの選択肢がある。

1. `CGEvent` タップ — アクセシビリティ権限が必要
2. `NSEvent.addGlobalMonitorForEvents` — アクセシビリティ権限が必要
3. `Carbon RegisterEventHotKey` — アクセシビリティ権限不要

我々は3を選んだ。Carbon APIは古いが、ホットキー登録だけならアクセシビリティ権限を要求しない。これはユーザー体験上きわめて重要だ。「システム環境設定を開いてアクセシビリティ権限を付与してください」というオンボーディングは、一般ユーザーの離脱率が40%を超える。

```swift
// KoeHotKeyManager.swift
import Carbon

final class KoeHotKeyManager {
    private var hotKeyRef: EventHotKeyRef?
    private var eventHandler: EventHandlerRef?

    func register(keyCode: UInt32, modifiers: UInt32) {
        var hotKeyID = EventHotKeyID(
            signature: OSType(0x4B4F4500), // "KOE\0"
            id: 1
        )

        // Carbon Hot Key 登録 — アクセシビリティ権限不要
        RegisterEventHotKey(
            keyCode,
            modifiers,
            hotKeyID,
            GetApplicationEventTarget(),
            0,
            &hotKeyRef
        )

        // イベントハンドラ
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )

        InstallEventHandler(
            GetApplicationEventTarget(),
            { _, event, _ -> OSStatus in
                NotificationCenter.default.post(
                    name: .koeHotKeyPressed, object: nil
                )
                return noErr
            },
            1, &eventType, nil, &eventHandler
        )
    }

    deinit {
        if let ref = hotKeyRef {
            UnregisterEventHotKey(ref)
        }
    }
}
```

デフォルトのホットキーは `Cmd+Shift+K`。ユーザーがカスタマイズ可能で、設定は `UserDefaults` に保存する。

## macOS/iOS: CoreML Whisper

Apple Silicon搭載デバイスでは、WhisperモデルをCoreML形式に変換して使う。`whisper.cpp` のCoreMLバックエンドを利用する方法もあるが、純粋なCoreMLモデルのほうがNeural Engineの活用効率が高い。

モデルサイズと精度のトレードオフ:

| モデル | サイズ | 認識精度(日本語) | 処理速度(M3) |
|--------|--------|-----------------|-------------|
| Tiny | 75MB | 78% | 0.3x realtime |
| Base | 142MB | 84% | 0.5x realtime |
| Small | 466MB | 89% | 1.2x realtime |
| Large V3 | 1.5GB | 96% | 3.1x realtime |

macOSではLarge V3をデフォルトにしている。iOSではデバイスのRAMに応じてSmallまたはBaseに自動切り替えする。

## Windows: Rust + whisper.cpp + CUDA

Windowsでは Swift が使えないため、Rust で実装した。音声認識には `whisper.cpp` のRustバインディング `whisper-rs` を使い、CUDAまたはVulkanバックエンドで推論する。

```rust
// koe-windows/src/transcribe.rs
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

pub struct KoeTranscriber {
    ctx: WhisperContext,
}

impl KoeTranscriber {
    pub fn new(model_path: &str, use_gpu: bool) -> Result<Self> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(use_gpu); // CUDA or Vulkan

        let ctx = WhisperContext::new_with_params(model_path, params)
            .map_err(|e| anyhow!("Whisper init failed: {e}"))?;

        Ok(Self { ctx })
    }

    pub fn transcribe(&self, pcm_16khz: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("ja"));
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        // VAD: 無音区間の検出閾値
        params.set_no_speech_threshold(0.6);

        let mut state = self.ctx.create_state()
            .map_err(|e| anyhow!("State creation failed: {e}"))?;

        state.full(params, pcm_16khz)
            .map_err(|e| anyhow!("Transcription failed: {e}"))?;

        let n_segments = state.full_n_segments()
            .map_err(|e| anyhow!("Segment count failed: {e}"))?;

        let mut result = String::new();
        for i in 0..n_segments {
            if let Ok(text) = state.full_get_segment_text(i) {
                result.push_str(&text);
            }
        }

        Ok(result)
    }
}
```

CUDA対応GPUがない環境では、Vulkanフォールバックを使う。それも不可の場合はCPU推論になるが、Large V3だとリアルタイム処理が困難なのでSmallモデルに自動ダウングレードする。

## iOS: Soluna P2P分散推論

iOSではデバイスのスペック制約から、LLMによるテキスト補正をローカルで実行するのが難しい。ここでSolunaのP2Pネットワークを活用する。同一ネットワーク上のmacOSデバイスやデスクトップマシンに推論を委譲する。

Bonjourプロトコルでローカルネットワーク上のKoeノードを自動検出し、最もスペックの高いノードにLLM推論をオフロードする。P2Pノードが見つからない場合は、テキスト補正なしの生認識結果をそのまま出力する。

## バイナリサイズ最適化

配布サイズは重要だ。特にmacOS版はDMGでの配布を想定しており、100MBを超えるとダウンロード完了率が大幅に低下する。

最適化施策:
- CoreMLモデルは `mlmodelc` (コンパイル済み) でバンドルし、初回起動時のコンパイルを省略
- Whisper Large V3は初回起動時にダウンロード (アプリ本体には含めない)
- Rustバイナリは `strip` + `opt-level = "z"` + `lto = true` で最小化
- Windows版: 12.4MB (モデル除く)、macOS版: 8.7MB (モデル除く)

## まとめ

3プラットフォーム対応で最も苦労したのは「同じ体験を異なる技術スタックで実現する」ことだ。macOSとiOSはSwiftで共有できるコードが多いが、WindowsはRustで完全に書き直す必要がある。共通化できるのはプロトコル仕様とモデルファイルのみ。

Carbon Hot Key APIの選択は地味だが重要な判断だった。アクセシビリティ権限不要という一点で、オンボーディングの摩擦が劇的に減る。技術的には古いAPIだが、ユーザー体験を優先した合理的な選択だと考えている。
