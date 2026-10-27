---
title: "LINE巡回のAI判定をローカルの決定モデルに置き換えた — 無料・0.45秒・オフライン可"
date: 2026-10-27
tags: [tech, ollama, nimble, ai-agent, sente]
description: "SenteのLINE巡回スキルで使っている有料Jev APIによる応答要否判定を、ローカルのOllama決定モデル（nimble）で実行できるようにした記録。遅延・コスト・精度の実測比較と、確信度キャリブレーションの違いへの対応。"
---

# LINE巡回のAI判定をローカルの決定モデルに置き換えた

Sente（私のAI秘書）のLINE巡回では、新着メッセージへの対応要否をAIが3択（返答不要 / AI事実回答 / 本人エスカレーション）で判定しています。これまで有料のJev API（TypeSafeの決定モデル）を使っていましたが、**ローカルのOllamaで動く決定モデル（nimble）に置き換えられるようにしました**。

- コスト: クレジット消費 → **ゼロ**
- 遅延: ネットワーク往復1〜3秒 → **0.45秒/件**
- オフライン: 不可 → **可**
- 安全設計: 確率0.9未満は `human`（本人通知）に倒すゲートは**そのまま維持**

## 背景: 「決定モデル」という新しいレイヤー

2026年後半のAIエージェント動向で大きな流れのひとつが、**低遅延の意思決定レイヤー**です。

- **OpenAI Decisions API (GA, 10/22)**: 分類・ルーティング・次アクション選択を専用API化。Sierraでp95 280ms・コスト90%減
- **Ollama が Jev-style decision models 対応 (9/29)**: 同じ思想の決定モデルをローカル・無料・低遅延で実行可能に
- **Claude Haiku 5.5 (10/7)**: サブエージェント・ブラウザ操作向け最速モデル

エージェントがワークフロー中に行う「小さな判断」（分類・振り分け・継続/再試行/質問の選択）を、フルサイズのLLMではなく専用の軽量モデルに任せる設計が一般化してきました。

## 対象: takibi-line-watch の Jev 判定

Senteの `takibi-line-watch` スキルでは、TAKIBIチャネルのLINE新着を巡回し、各メッセージを3択で判定しています。

| 判定 | 動作 |
|---|---|
| `no_reply` | 雑談・リアクション・回答済み → 返信しない |
| `ai_reply` | 検証済み事実で回答できる質問 → AIが短く事実回答 |
| `human` | 購入・承諾・日程・曖昧 → 本人へ通知 |

従来は teai 経由の有料Jev API（`typesafe/jev`）を呼んでおり、1巡回あたり40クレジットの上限を設けていました。

## 実装: `--local` オプションを追加

`judge.py` に `--local` フラグを追加し、有料APIの代わりにローカルOllamaの `/v1/systemone` エンドポイントを叩くようにしました。

```python
# 従来（有料Jev API）
python3 judge.py --live --run-id <巡回ID> < 入力JSON

# 新規（ローカル・無料）
python3 judge.py --local < 入力JSON
```

リクエスト/レスポンスの構造（`state` + `questions` + `choice/probabilities/confidence`）はJev APIと互換があるため、既存の検証ロジック（`decide()`）をそのまま再利用できました。

## 詰まったところ: nimbleのconfidenceはJevと尺度が違う

実測で重要な発見がありました。**nimbleはchoice確率が0.95でもconfidenceが0.45と低く出る**ケースがあります。

```json
{
  "choice": "ai_reply",
  "probabilities": {"ai_reply": 0.95, "no_reply": 0.03, "human": 0.02},
  "confidence": 0.45
}
```

Jevでは確率とconfidenceが連動していましたが、nimbleでは独立した値として出ます。従来の `confidence < 0.9 → human` ゲートをそのまま適用すると、明確なケースまで `human` に倒れてしまいます。

**対応**: ローカルでは**choice確率のみをゲートに使い**、confidenceは記録のみにしました。確率0.9未満は従来通り `human` に倒す安全設計は維持しています。

```python
if local:
    if probabilities[choice] < 0.9:
        choice, reason = "human", "low_confidence"
elif answer["confidence"] < 0.9 or probabilities[choice] < 0.9:
    choice, reason = "human", "low_confidence"
```

## 実測比較

| 項目 | 有料Jev API | ローカルnimble |
|---|---|---|
| 遅延 | 1〜3秒（ネットワーク往復） | **0.45秒/件** |
| コスト | クレジット消費（上限40/巡回） | **ゼロ** |
| オフライン | 不可 | **可** |
| 判定精度 | 確率+confidence連動 | 確率ゲートのみ |
| 安全設計 | 確率0.9未満→human | **同じ（維持）** |

テストケースでの判定結果：

- 「ラジオの新しいエピソードはいつ？」（明確・証拠あり）→ `ai_reply`（確率0.99）
- 「ありがとう！」→ `no_reply`
- 「3万円で買っていい？」→ `human`（購入判断ゲート）
- 「直りましたか？」（証拠不足）→ `human`（証拠欠損ゲート）

曖昧なケースは従来通り `human` に倒れ、誤った自動返信を防ぐ設計が維持されています。

## セットアップ

```bash
# Ollama 0.40.11以降が必要（nimbleのpullに必要）
brew upgrade ollama

# nimbleモデルをpull
ollama pull nimble

# serve起動
ollama serve  # localhost:11434
```

## まとめ

- エージェントの「小さな判断」をローカルの決定モデルに置き換えることで、**コストゼロ・低遅延・オフライン可**を実現できた
- nimbleのconfidenceはJevと尺度が異なるため、**確率ゲートのみ適用**する調整が必要だった
- 有料Jevは残してあり、重要な巡回では切り替え可能。まずはローカルで運用して精度を観察する

「決定モデル」は今後もエージェント設計の重要なレイヤーになっていくと思います。高頻度の軽判断をフルモデルに回している場合は、ローカル決定モデルの導入を検討する価値があります。
