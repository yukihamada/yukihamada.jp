---
title: "Koe: 1週間でmacOS→Windows→iOSの音声入力アプリを作った"
date: 2026-03-10
description: "v1.0からv2.5まで1週間。CoreML、whisper.cpp、llama.cpp、Metal GPU。クラウド不要のローカルAI音声入力アプリKoeの開発記録。"
tags: ["Koe", "音声", "マルチプラットフォーム"]
---

> **技術詳細は別記事に分けた** → [Koe技術スタック — CoreML + whisper.cpp + llama.cpp + Metal](/blog/2026-03-10-koe-cross-platform-tech)

## 「自分が毎日使いたいもの」を作った

Koeは、音声をテキストに変換するアプリだ。ただし、クラウドに音声を送らない。すべてデバイス上で処理する。

作った動機はシンプルで、自分が毎日使いたかったからだ。コードを書きながらメモを取りたい、会議の内容をサッと文字起こししたい、でもOpenAIやGoogleに音声データを送りたくない。特に仕事の会議内容は、外部に出したくないことが多い。

既存のローカル音声認識ツールは、どれも使い勝手がイマイチだった。CLIで動くものはあるけど、普段使いには不便すぎる。だったら自分で作ろう、となった。

## Day 1-2: macOS版（v1.0）

最初にmacOS版を作った。SwiftUIでメニューバー常駐アプリにして、ショートカットキー（⌘+Shift+K）で録音開始・停止。録音が終わるとテキストがクリップボードに入る。

音声認識にはAppleのSpeech frameworkではなく、**CoreML版のWhisper**を使った。理由は精度と多言語対応。AppleのSpeech frameworkは日本語の認識精度がいまいちで、技術用語にも弱い。Whisper large-v3をCoreMLに変換したモデルなら、「axum」も「tokio」も正しく認識してくれる。

CoreMLモデルのサイズは約1.5GB。初回ダウンロードは重いけど、一度入れてしまえばオフラインで動く。認識速度はM3 MacBook Proで、1分の音声を約8秒で処理。実用上まったく問題ない。

## Day 3-4: Windows版（v1.5）

macOS版が動いたので、次はWindowsだ。SwiftはWindowsで動かないので、Rustで書き直した。

音声認識エンジンは**whisper.cpp**を使った。ggml形式のWhisperモデルをRustから呼び出す。UIはegui（Rust製のGUIフレームワーク）で、タスクトレイ常駐アプリにした。

一番苦労したのはオーディオキャプチャだ。macOSではAVAudioEngineで簡単にできることが、WindowsではWASAPI（Windows Audio Session API）を直接叩く必要がある。cpalクレートを使ったけど、デバイス切り替え時のクラッシュに悩まされた。最終的にはオーディオスレッドを分離して、デバイス変更を検知したらストリームを再初期化する形で安定させた。

バイナリサイズも課題だった。whisper.cppを静的リンクすると、モデルなしでも実行ファイルが45MBになる。UPXで圧縮して18MBまで縮めた。

## Day 5-6: iOS版（v2.0）+ テキスト要約

iOS版はmacOS版のSwiftUIコードをほぼ流用できた。CoreMLモデルも同じ。ただし、iPhoneのストレージ事情を考えて、モデルサイズの小さいWhisper base（139MB）をデフォルトにして、large-v3はオプションダウンロードにした。

ここで新機能を追加した。**テキスト要約**だ。音声認識で得たテキストを、llama.cppで動くローカルLLMに渡して要約する。会議の文字起こしが自動で箇条書きになる。

モデルはQwen2.5-3B-Q4_K_M。iPhoneのNeural Engineで動かすと、1000文字の要約に約5秒。「会議で決まったことリスト」が自動で出てくるのは、思った以上に便利だった。

## Day 7: Metal GPU最適化（v2.5）

最終日は最適化に使った。macOS版で、Metal GPUを使ったwhisper.cppの推論高速化を実装。CPUだけだと1分の音声に8秒かかっていたのが、Metal GPUで3.2秒まで縮んだ。

## 1週間の成果

- **macOS版**: SwiftUI + CoreML Whisper。メニューバー常駐
- **Windows版**: Rust + whisper.cpp + egui。タスクトレイ常駐
- **iOS版**: SwiftUI + CoreML Whisper + llama.cpp。テキスト要約付き
- **v1.0→v2.5**: 7バージョンリリース、コミット数142

全プラットフォームで共通しているのは、**音声データが一切外部に出ない**ことだ。Wi-Fiを切っていても動く。飛行機の中でも使える。

個人的に一番気に入っているのは、macOS版のショートカットキーで録音→クリップボードの流れ。コードを書きながら「ここの処理はユーザー認証のトークンを検証してから...」と喋るだけで、コメントのドラフトがクリップボードに入る。自分の開発体験が変わった。

---

技術的な詳細はこちら → [Koeのクロスプラットフォーム技術詳細](/blog/2026-03-10-koe-cross-platform-tech)
