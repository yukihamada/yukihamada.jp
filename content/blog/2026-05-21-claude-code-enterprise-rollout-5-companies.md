---
title: "Claude Code を会社で配るとき、 何が正解か ─ メルカリ・楽天・ZOZO・GMO・Gemcook 5 社の事例から見えた 2026 年の指針"
date: 2026-05-21
tags: [claude-code, AI, enterprise, mercari, rakuten]
description: "2026 年 4-5 月に日本の主要 IT 企業から Claude Code 全社展開事例が一斉に出始めた。 メルカリの MDM 配布、 楽天の 79% 期間短縮、 ZOZO の Plugin × MCP、 GMO の全エンジニア展開、 Gemcook の段階導入 ─ 5 社を比較して、 2026 年版の "配り方" の正解を整理する。"
---

2026 年 4-5 月、 日本の主要 IT 企業から Claude Code の全社展開事例が一斉に出始めた。 メルカリの MDM 配布戦略、 楽天の `79% 期間短縮`、 ZOZO の Plugin × MCP、 GMO デザインワンの全エンジニア展開、 Gemcook の 3 年越し導入記録 ─ 同じツールを使っていながら、 各社の "配り方" が驚くほど違う。

このブログでは、 5 社の事例を整理しながら **「全社配布で何が正解か」** を 2026 年 5 月時点の最新情報で整理する。 一企業のやり方を真似するのではなく、 **自社の文脈に合わせた組み立て方** が見えてくることを目指したい。

---

## 1. 5 社の事例サマリー

### 🟥 メルカリ ─ MDM 二層配布で 数百名へ

- **配布対象**：エンジニア ＋ **非エンジニア 全社員（数百名）**
- **手法**：MDM（Jamf/Kandji/Intune）で `managed-settings.json` を強制配布
- **キー設定**：`bypassPermissionsDisabled: true` で `--dangerously-skip-permissions` を CLI 引数からも上書き不可
- **階層化**：エンジニアは customization 余地あり、 非エンジニアは `disableUserConfig: true` で完全ロック
- **成果**：**非エンジニア 1 人あたり月 10-15 時間 削減**

メルカリの肝は「配布」 そのものを工学化したこと。 数百名スケールでは、 個人の善意に依存できない。 MDM で OS レベルから enforce する設計は、 「個人で壊せない env を提供する」 という Defense-in-Depth の正しい実装。

### 🟧 楽天 ─ 並列実行で生産性を 5x

- **配布対象**：エンジニア全社 ＋ Claude Managed Agents で エンジニア・プロダクト・営業・マーケ・財務 横断
- **手法**：Slack / Teams 統合、 **複数 Claude Code セッション 並列実行**
- **成果**：機能納期 **24 日 → 5 日（79% 短縮）**、 7 時間連続 autonomous coding、 修正精度 **99.9%**
- **ガバナンス**："sandboxed environments" + "coding guidelines as safeguards"
- **語録**（梶 General Manager of AI for Business）：「5 タスクのうち 4 つを Claude Code に委任、 自分は 1 つに集中」

楽天の特徴は、 **「エンジニアの並列稼働」** を本気で前提にしていること。 1 人の人間が 5 Claude Code セッションを同時管理し、 ボトルネックを人間ではなく AI 側に押し込む。 マネージドエージェントを Slack/Teams 経由で非エンジニア部門にまで届けて、 開発を全社員に民主化している。

### 🟩 ZOZO ─ Plugin × MCP で ガイドライン準拠 自動化

- **配布対象**：エンジニア全社
- **手法**：Claude Code Plugins（再利用可能ワークフロー）× **Atlassian MCP**（Confluence の guideline 読み取り、 `READ_ONLY_MODE: true`）
- **アーキ**：tech stack 自動検出 → 該当ガイドライン抽出 → サブエージェント並列チェック → "46 項目 OK / 5 項目 NG＋具体的指摘" の報告
- **効果**：マーケットプレイス配布で **設定同期 / 更新反映** の運用コスト消滅

ZOZO の事例は、 「Claude Code が便利」 という話を超えて、 **会社固有の知識基盤（Confluence）と Claude を MCP で繋ぐ** という 2026 年型のアーキを示している。 ガイドラインが更新されたら次の `/guideline-review` 実行から即反映 ─ ドキュメントとレビューの同期問題が構造的に消える。

### 🟨 GMO デザインワン ─ 全エンジニア配布で 50% 効率化

- **配布対象**：エンジニア全社
- **目標**：開発効率 **50% 向上**
- **選定理由**：自然言語インタラクションの直感性、 実装意図を汲み取るコード提案精度

GMO の事例は最もシンプル。 「全エンジニアに渡す、 目標数値を立てる」 だけ。 シンプルさが武器で、 まず横展開してから運用設計を詰める段階アプローチ。

### 🟦 Gemcook ─ 3 年越しの段階導入

- **タイムライン**：2023/2 GitHub Copilot → 2024-25 各 agent 試行（Cline, Cursor, Windsurf, Devin） → 2026/2 Claude Code 全社展開
- **教訓**：「個人選択を許した期間が標準化を遅らせた」
- **判断軸**：実装品質、 エコシステム採用率、 Team plan の運用しやすさ
- **観察**：導入数日で個人が 数万行コード生成

Gemcook の最大の示唆は **「標準化を待ちすぎるな」**。 「明日もっといいツールが出るかもしれない」 を理由に決断を延ばすと、 結果として組織全体の AI リテラシーが揃わなくなる。 今日時点の最適解を選んで標準化し、 必要なら明日また選び直す ─ という プラグマティズム。

---

## 2. 5 社の事例から見えた 3 つの共通軸

事例を並べると、 全社が **同じ 3 つの軸** で意思決定していることが見えてくる。

### 軸 ①：**配布対象の幅**（誰に渡すか）

| 配布範囲 | 該当事例 | 必要なガバナンス強度 |
|---|---|---|
| エンジニアのみ | GMO、 ZOZO、 Gemcook | 低-中（個人責任 + チームレビュー） |
| エンジニア + プロダクト | 楽天 | 中（sandbox + guidelines） |
| **エンジニア + 非エンジニア 全社員** | **メルカリ** | **高（MDM enforcement 必須）** |

非エンジニアまで配ると、 個人の判断に依存できなくなり、 **環境レベルでの enforcement が必須**になる。

### 軸 ②：**自動実行の許容度**

Anthropic 公式データによると、 **ユーザーは permission prompt の 93% を承認**している（approval fatigue）。 これに対する各社の解決法：

| アプローチ | 該当事例 | 仕組み |
|---|---|---|
| 全承認制を維持 | （多くの企業） | 個人判断、 fatigue 発生 |
| Sandbox 内 bypass 許可 | 楽天（sandboxed env） | env レベルで安全担保 |
| **MDM で bypass フラグ無効化** | **メルカリ** | OS レベルで CLI 引数を遮断 |
| **Auto Mode**（2026 年新登場） | Anthropic 推奨 | AI classifier が自動判定、 false-negative 率 17% |

### 軸 ③：**コンテキスト配布の構造化**

5 社全てが、 単に Claude Code を入れただけでなく、 **会社固有のコンテキストを Claude に与える仕組み** を作っている：

| 手法 | 事例 |
|---|---|
| `CLAUDE.md` を MDM 配布 | メルカリ |
| Claude Managed Agents を Slack/Teams 統合 | 楽天 |
| MCP + Plugin マーケットプレイス | ZOZO |
| Team plan + 仕様書 workflow 統合 | Gemcook |

**Claude Code 自体ではなく、 "自社の知識を Claude に翻訳する層" の設計こそが差別化要因**になっている。

---

## 3. 「正解」 の組み立て方 ─ 自社の文脈で選ぶ

5 社の共通項から、 自社向けの "配り方" を組み立てる 4 ステップの指針。

### Step 1：配布対象を 3 ティアに分割

```
Tier A：シニアエンジニア（10-30 名）
  → 個人 devcontainer / sandbox、 bypass フラグ自己責任 OK
  → 速度最優先、 Auto Mode + 信頼コードで作業

Tier B：一般エンジニア（30-300 名）
  → Auto Mode 標準、 共有 devcontainer、 PreToolUse hooks で危険コマンド遮断
  → MCP + Plugin で社内コンテキスト自動注入

Tier C：非エンジニア（数百-数千名）
  → MDM enforcement、 bypassPermissionsDisabled: true、 disableUserConfig
  → 限定された tool set のみ許可、 Slack/Teams フロントで触らせる
```

### Step 2：環境境界を OS / コンテナで実装

- **Tier A**：個人 Docker / devcontainer、 firewall は自由
- **Tier B**：会社配布 devcontainer + egress 制限（GitHub / npm / Claude API のみ）
- **Tier C**：MDM 強制設定（CLI 引数で上書き不可な層）

「個人の善意」 を信頼境界にしない ─ これは 2026 年の合意事項。

### Step 3：Auto Mode を デフォルトに

2026 年 4 月リリースの **Auto Mode** は、 「bypass の代替」 として設計された：

- AI classifier が安全な action を自動承認
- approval fatigue 解消
- false-negative 率 17%（残リスク認識必須）
- **本番 infra や重要システムには使わない**

通常開発は Auto Mode、 本番 deploy は manual review、 sandbox 内検証は bypass、 という 3 階層が標準形になりつつある。

### Step 4：会社固有コンテキストを構造化

- **CLAUDE.md**：会社のコーディング規約・禁止事項・お作法
- **MCP**：社内 wiki / Jira / Confluence への安全な read アクセス
- **Plugins**：再利用可能な workflow（ZOZO の guideline-review がモデル）
- **Skills**：頻出タスクをスキル化（30 dev shop 事例：22 個の shared skills で月 4 で 35% 生産性向上）

---

## 4. ROI 数字で見る投資判断

| 事例 | 投資 | 効果 |
|---|---|---|
| メルカリ | MDM 設定工数 + Team plan ライセンス | 非エンジニア月 10-15h 削減 × 数百名 |
| 楽天 | Claude Code + Managed Agents 全社 | 機能納期 79% 短縮、 並列 5x |
| 30 dev shop（US） | 1 quarter の rollout | 月 4 で **生産性 35% 向上維持** |
| Anthropic 全体 | ─ | 2026 ビジネス subscribers 4x、 ARR $2.5B 超 |

「コーディング作業の効率化」 だけ見ると物足りないが、 **"非エンジニアにもコード生成能力を配る"** という民主化視点で見ると、 月 10-15h × 数百名 = 数千時間/月 の構造的レバレッジになる。

---

## 5. 結論 ─ 2026 年の "配り方" の本質

5 社の事例が示しているのは、 ある明確なシフトだ：

> **2026 年の Claude Code 全社展開は、 「使うこと」 ではなく 「配ること」 が主戦場になっている。**

ツール自体の能力は安定してきた。 差別化は、

- **誰に**（ティア設計）
- **どこまで権限を与え**（環境境界）
- **何を Claude に教え**（CLAUDE.md / MCP / Plugin）
- **どう運用を続けるか**（retro、 audit、 Skill 育成）

の 4 点に移った。 メルカリの MDM、 楽天の並列、 ZOZO の Plugin × MCP、 GMO の全配布、 Gemcook の段階導入 ─ どれも「正解」 であり、 同時に「自社の文脈での選択」 だ。

自社で Claude Code 展開を考えるとき、 まず問うべきは：

1. **誰に配るか？**（エンジニア限定？ 非エンジニア込み？）
2. **どこを信頼境界にするか？**（個人？ コンテナ？ MDM？）
3. **どんなコンテキストを Claude に与えるか？**（README で済む？ MCP 必要？）
4. **誰が Skill / Plugin を育てるか？**（最も投資意欲ある エンジニアに air cover）

この 4 つに答えれば、 5 社のどの形が自社に近いか、 あるいは 5 社の組み合わせが必要か、 自ずと見えてくる。

Claude Code は「導入して終わり」 ではなく、 「育てて続ける」 プラットフォーム。 2026 年は、 その運用設計の試行錯誤が一斉に始まった年として記録されるだろう。

---

## Sources / 参考資料

**事例**

- [メルカリの Claude Code セキュリティ設定の組織配布戦略 ─ Speaker Deck](https://speakerdeck.com/hi120ki/claude-code-organization-settings)
- [メルカリの Claude Code セキュリティ設定を参考に金融業界向けの方針を考えた ─ Zenn](https://zenn.dev/nekoai_lab/articles/fe200eacaf70ae)
- [メルカリが示した「全社員に AI を配る」作法 ─ SYNCON](https://syncon.jp/mercari-claude-code-mdm-strategy/)
- [メルカリ数百名 Claude Code 全社展開の全手順 2026 年 4 月版 ─ note](https://note.com/shiodome_098/n/ncebfb52943e9)
- [Rakuten Customer Story ─ Anthropic 公式](https://claude.com/customers/rakuten)
- [ZOZO ─ Claude Code Plugins × Atlassian MCP で ガイドライン準拠チェック](https://techblog.zozo.com/entry/check-guideline-with-claude-code-plugins)
- [GMO デザインワン、 全エンジニアに『Claude Code』導入](https://group.gmo/news/article/9794/)
- [Gemcook ─ Claude Code 全社導入までの意思決定と歴史](https://zenn.dev/gemcook/articles/claude-code-company-wide)
- [Case Study: Claude Code Adoption at a 30-Dev Shop 2026 ─ Digital Applied](https://www.digitalapplied.com/blog/case-study-claude-code-team-adoption-30-dev-shop-2026)

**公式**

- [Claude Code Auto Mode ─ Anthropic 公式 Engineering](https://www.anthropic.com/engineering/claude-code-auto-mode)
- [Development Containers ─ Claude Code Docs](https://code.claude.com/docs/en/devcontainer)
- [Anthropic GitHub Issue #19978 ─ Best Practices vs Devcontainer 矛盾](https://github.com/anthropics/claude-code/issues/19978)

**業界トレンド**

- [Anthropic finally beat OpenAI in business AI adoption ─ VentureBeat](https://venturebeat.com/technology/anthropic-finally-beat-openai-in-business-ai-adoption-but-3-big-threats-could-erase-its-lead)
- [How to sandbox AI agents in 2026: MicroVMs, gVisor ─ Northflank](https://northflank.com/blog/how-to-sandbox-ai-agents)
- [Sandboxed Environments for AI Coding ─ Bunnyshell](https://www.bunnyshell.com/guides/sandboxed-environments-ai-coding/)
