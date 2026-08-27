---
title: "自分の曲をAIでアフロハウスに変換してみた — ACE-Stepとaudio-to-audio"
date: 2026-08-27
tags: [music, edm, afro-house, ai, ace-step]
description: "オープンソースの音楽生成モデルACE-Stepを使い、fal.ai経由で自分の曲『Koe』のテーマ曲をアフロハウス風にリミックスした実験の記録。"
---

# 自分の曲をAIでアフロハウスに変換してみた

[前回](/blog/2026-08-27-afro-house-explained)[前々回](/blog/2026-08-27-afro-house-scene-artists)とアフロハウスについて書いてきたので、今度は実際に手を動かしてみることにした。自分の曲をAIでアフロハウス風に作り変えられないか、という実験。

## ACE-Stepとは

オープンソースの音楽生成モデル**ACE-Step**を使うことにした。Apache 2.0ライセンスで、商用利用も明示的にOK。ローカルでも動く軽さが売りで、A100なら1曲2秒未満、RTX 3090でも10秒程度、4GB VRAMがあれば動くと謳っている。

今回は**fal.ai**という、こうしたオープンソースモデルをAPIとしてホストしてくれるサービス経由で叩いた。

## audio-to-audioで「自分の曲」をアフロハウス化

ACE-Stepには`text-to-audio`だけでなく`audio-to-audio`というエンドポイントがある。既存の音源をURLで渡すと、その曲の雰囲気を保ちながらジャンルだけを変換してくれる機能。

これを使えば「僕の曲をアフロハウスにする」がそのまま実現できる。素材に選んだのは、[Koe](https://koe.live)のテーマ曲(`koe_song.mp3`)。

```bash
curl -X POST "https://queue.fal.run/fal-ai/ace-step/audio-to-audio" \
  -H "Authorization: Key $FAL_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "audio_url": "https://yukihamada.jp/mv/koe_song.mp3",
    "original_tags": "j-pop, ballad, emotional, male vocal, piano, mid-tempo",
    "tags": "afro house, 121 bpm, log drum, polyrhythmic percussion, deep bass, tribal, four on the floor, dance",
    "lyrics": "[inst]"
  }'
```

fal.aiはキュー式のAPIで、投げるとジョブIDが返り、`/status`をポーリングして完了を待つ。今回は推論に199秒かかり、2分7秒のWAVファイルが出てきた。

<audio controls preload="none" style="width:100%;margin:1em 0" src="/audio/blog/koe-song-afrohouse.mp3"></audio>

聴いてみると、元曲のメロディの輪郭は残しつつ、ビートとパーカッションが完全にアフロハウスの語彙に置き換わっている。「自分の曲のアフロハウスカバー」がAI一発で出てくる時代になったんだな、というのが率直な感想。

## コストと所要時間

- テスト生成(20秒・text-to-audio): 推論2.5秒
- 本番生成(2分7秒・audio-to-audio): 推論199秒
- コストは1回あたり数セント〜十数セント程度。実験用途としては十分安い

## わかったこと

- **audio-to-audioは「学習」ではなく「変換」**。LoRAで自分の曲調そのものをモデルに覚え込ませるのがフル装備のゴールだが、fal.aiのホスト型APIではそこまでの学習ジョブは提供されていない(自前でGPUを持ってセルフホストする必要がある)。今回はその手前の「既存の曲をジャンル変換する」で実用上十分な結果が出た
- 生成AIの民主化がこのレベルまで来ると、「好きな曲を好きなジャンルで聴き直す」がリスナー側の遊びとしても普通になっていきそうだ

---

**関連**
- [アフロハウスとは何か — リズムの理論から世界的ブレイクまで](/blog/2026-08-27-afro-house-explained)
- [アフロハウスは今どう回っているのか — アーティスト別に見る5つのレーン](/blog/2026-08-27-afro-house-scene-artists)
- [ACE-Step — GitHub](https://github.com/ace-step/ACE-Step)
- [fal.ai ACE-Step Audio to Audio API Docs](https://fal.ai/models/fal-ai/ace-step/audio-to-audio/api)
