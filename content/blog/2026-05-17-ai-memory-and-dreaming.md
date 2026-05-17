---
title: "AIに「経験値」を貯めさせる：Memory・Multi-agent shared memory・Dreaming を、自分のエージェント群で実装してみた"
date: 2026-05-17
tags: [claude-code, Anthropic, Memory, Dreaming, OpenClaw, mini-agent-c, エージェント, AI開発]
description: "Anthropic が発表した『AIに経験値を貯める』3層 — Memory / Multi-agent shared memory / Dreaming。自分の Claude Code auto-memory、OpenClaw fleet、mini-agent-c v11、chatweb.ai に当てはめてやり方を事細かに書く。最後に Opus が考えるベストプラクティスを置く。"
---

Anthropic の Platform Team PM である Mahesh が「**AIに経験値を貯めて進化させる方法**」を公開した（2026-05-16）。要点だけ言うと、AI エージェントに「**毎回ほぼ初対面**」をやめさせる仕組みだ。

（英語の発表全文は別ページに置いた → [[Transcript] Mahesh @ Anthropic — Memory and Dreaming for self-learning agents](/blog/2026-05-17-anthropic-memory-talk-transcript)）

Memory は数週間前に **cloud-managed agents で public beta**、Dreaming は **本日 research preview**（Managed Agents API）として出ている。Anthropic 自身、社内に**数百〜数千のエージェントが同じ memory を共有しながら同時に走っている**前提で設計されている、と Mahesh は話していた。

これは僕がいま並行で走らせている AI 系プロダクト全部に効く話で、すでに局所的にやっていることでもある。だから棚卸しを兼ねて、**自分のエージェント群に当てはめながら、やり方を事細かに書く**。最後に Opus が考えるベストプラクティスを置く。

## まず3層をひとことで

| 層 | やっていること |
|---|---|
| ① **Memory** | AI が作業中に学んだ「成功条件・失敗・環境知識・調査結果」をファイル型の記憶に書き出して、次回のセッションが読む |
| ② **Multi-agent shared memory** | 複数の AI が同じ Memory を読み書きする。権限管理・並行制御・履歴管理つきで、チーム全体に集合知が貯まる |
| ③ **Dreaming** | バックグラウンドで全エージェントの作業ログを横断分析し、重複の統合・古い記憶の削除・パターン発見を自動で回す。**翌日のエージェントが昨日より賢い** |

「AI を使う」から「AI を育てる」への移行、と言ってもいい。

### 実例の効きが既に出ている

Mahesh が紹介した実例が示唆的だった。

- **Rakuten** — 社内ナレッジエージェントに memory を入れたら、**初回ミスが 90% 減**。失敗が次の世代に引き継がれるので、同じバグを繰り返さない。結果としてトークン効率も上がり、コストもレイテンシも下がった
- **Harvey**（法務 AI）— 法務シナリオの benchmark で Dreaming を入れたら、**タスク完了率が 6 倍**に増えた

90% と 6 倍は、プラットフォームの差というより**「経験が貯まる側 vs 貯まらない側」の差**だと思っている。

## なぜこれが効くのか — 「毎回ほぼ初対面」の壁

これまでのエージェントは、毎回プロンプトに以下を人間が書く必要があった。

- このリポジトリのコードベースの癖
- 前回どの調査までやったか
- どの戦略が成功して、どこで失敗したか
- 触ってはいけないファイル・触っていいファイル
- 他のエージェントが学んだこと

僕は Claude Code を 10〜15 セッション並列で回す日もあるが、これを毎回口頭で渡すのは現実的じゃない。だから「**書き留めておいて、未来の自分（次のセッション）に渡す**」の仕組みを、すでに局所的に運用している。それを Anthropic が**プラットフォームの一級市民として標準化した**、というのが今回の発表の本質だ。

## 自分のエージェント群に当てはめる

### ① Claude Code auto-memory（個人ローカル）

僕の `~/.claude/projects/.../memory/` には今 **74 ファイル**ある。中身を 4 タイプに分けている。

```
memory/
├── MEMORY.md                              # インデックス（常時読み込まれる）
├── user_profile.md           type: user   # 「Mercari CPO → NAH → Enabler」など前提
├── product_philosophy.md     type: user   # 「速く、ノイズなく」が世の中の摂理
├── feedback_chatweb_deploy.md type: feedback # Lambda は削除済み。絶対に fly deploy しない
├── feedback_pii_protection.md type: feedback # 公開記事で実名・email 晒さない
├── openclaw_fleet.md         type: project # 4台の VPS と Telegram bot 構成
├── nemotron_pods.md          type: project # RunPod の URL とパラメータ
└── cloudflare_dns.md         type: reference # ゾーン ID と CLI コマンド
```

4 タイプの使い分けはこう。

- **user**：僕がどういう前提知識・役割で動いているか。「Rust 自作派」「柔術青帯」「議論の前提」など。エージェントが説明の粒度を合わせるのに使う
- **feedback**：「**こう書かれたら次から従う**」というルール。**Why** と **How to apply** をセットで書くのが鉄則。理由を書いておくと、エッジケースで自分で判断できる
- **project**：「いま走っている案件」「誰が何を、なぜ、いつまでに」。寿命が短いので、新情報で上書きしていく
- **reference**：「外部システムの場所」。Linear のプロジェクト名、Grafana の URL、DNS の管理 CLI など

#### やり方の具体

新しい memory を書くときは必ず**2 ステップ**。

1. **本体ファイル**（例：`feedback_chatweb_deploy.md`）に frontmatter つきで書く

   ```markdown
   ---
   name: chatweb.ai deploy target
   description: chatweb.ai 本番は Fly.io chatweb-ai のみ。Lambda は削除済み
   type: feedback
   ---
   chatweb.ai の本番は Fly.io アプリ `chatweb-ai` のみ。

   **Why:** Lambda + API Gateway の運用コストを排除して Fly.io に一本化。
   **How to apply:**
   - デプロイ: `fly deploy -a chatweb-ai --remote-only`
   - `deploy-fast.sh` は絶対に使わない
   ```

2. **MEMORY.md に 1 行追記**

   ```
   - **[feedback_chatweb_deploy.md](feedback_chatweb_deploy.md)** — chatweb.ai 本番は Fly.io のみ。Lambda 削除済み（2026-03-19）
   ```

MEMORY.md は**常時読まれるインデックス**で、200 行を超えると先頭しか読まれない。だから本文を MEMORY.md に書いてはいけない。**索引と本体を分ける**。これは Anthropic の Memory tool の設計と同じ。

これだけで、新しいセッションが立ち上がった瞬間「`chatweb-ai` には絶対 Lambda にデプロイしない、Fly.io 一択」という前提を持って動き始める。1 回も口頭で言わなくていい。

ちなみに Mahesh は「**Claude Opus 4.7 は file-system-based memory で SOTA**」と言っていた。実際、Opus 4.7 は「何を memory に書くべきか・どう構造化するか・何ファイルに分割するか」の判断が露骨に良くなっている。これは僕も毎日の Claude Code で体感している。Bash と Grep だけで memory を回せるのは、コーディングエージェントの延長で memory がうまく動くということでもある。

### ② OpenClaw fleet（multi-agent shared memory の予行演習）

僕は Hetzner の VPS 4 台に、性格の違う AI エージェントを常駐させている。

| エージェント | 役割 | Telegram |
|---|---|---|
| **Hachi 🐝** | 総合アシスタント | @yukihamada_ai_bot |
| **Kuro ⚡** | 技術特化 | @yukihamada_Codex_Openclaw_bot |
| **Ichi 1️⃣** | enablerdao.com / yukihamada.jp 専任 | @Enabler_Bossdog_bot |
| **Ni 2️⃣** | インフラ・セキュリティ特化 | @yukihamada_codexclaw_bot |

各 VPS は `~/.openclaw/workspace/` に独立した `MEMORY.md` を持っていて、30 分ごとに heartbeat で自己更新している。これが**ローカル Memory**。

ただ、いま致命的に欠けているのが「**Multi-agent shared memory**」だ。Hachi が enablerdao.com を調査して得た知見を、Ichi が読めない。Kuro が見つけたインフラの罠を、Ni が知らない。

Anthropic が言っていた「権限管理・並行制御・履歴管理つきの shared memory」が来たら、まずやるのはここだ。具体的には、

- **権限スコープ**：1 つのエージェントが、ある memory store に対しては read-only、別の store には read-write、というふうに混ぜられる。組織共通の runbook は全員 RO、個別タスクの working memory は RW、というのが基本パターン
- **Optimistic concurrency**：何百ものエージェントが同時に同じ memory を触っても壊れない。content hash で「自分が読んだ時の状態」を持っておき、書く時に差し替え検証をする
- **Audit log + attribution**：誰が・いつ・どのセッションが、どこを変えたかが全部追える。エージェントが過去の audit log を読んで「**最近この memory を誰がどう編集したか**」を参考にすることもできる
- **Standalone Portable API**：PII スキャンや独自クリーンアップを外部パイプラインで実装できる。memory が外に出せないと、企業利用は無理

OpenClaw の shared memory は今、Git でゆるく同期しているだけだから、↑ がそのまま欲しい。

```
shared-memory/
├── enabler-corp/        # Ichi が主に書く、全員が読める
├── infra-tripwires/     # Ni が書く、Kuro が読む
├── prod-incidents/      # 全員 RW、incident response 用
└── tone-of-voice/       # Hachi が書く、Ichi がブログで参照
```

権限を分けないと、エージェントが互いの「**個人的なメモ**」まで読み始めてカオスになる。Anthropic が権限管理を最初から組み込んでいるのはここが本質だと思う。

### ③ mini-agent-c v11（ローカル LLM × Memory）

C 言語で書いた自律エージェントで、M5 Mac 上の Qwen3.5-122B をローカルで叩いている。**API コスト $0**。

これに Memory を入れる場合、ファイル I/O だけでいい。`memory/YYYY-MM-DD.md` に当日の作業ログを書き、`MEMORY.md` をインデックスにする。SQLite も DB も要らない。**ファイルシステムが Memory のバックエンド**になる。

Anthropic の Memory tool もファイルシステム型なのは、これが最も「壊れにくい」からだ。バックアップは `tar`、検索は `grep`、編集は `vi`。**運用が異常に簡単**。

### ④ chatweb.ai / Nemotron + マルチモデル

chatweb.ai は Fly.io の `chatweb-ai` から Nemotron 9B（RunPod pod）+ Groq + Gemini を切り替えて使っている。Memory を入れると何が変わるかというと、

- 「過去にこの user_id がどのモデルで失敗したか」を覚える
- 「このプロンプトパターンは Nemotron が強い、これは Groq に流す」を学ぶ
- 失敗したリトライ理由をログとして残し、Dreaming に渡す

特に **Dreaming が活きるのはここ**で、夜間に「過去 7 日の全 conversation を横断して、Nemotron が特定のトークンパターンで毎回 hallucinate する」みたいなパターンを自動発見してくれたら、モデル選定ロジックが翌日から賢くなる。

デモで Mahesh が見せた具体例が分かりやすかった。SRE エージェントの dispatch latency 問題で、Dreaming が「**この latency 問題は、上流の CPU スパイクのちょうど 60 秒後に毎回起きている**」というパターンを発見した話。個別の SRE エージェントは自分のセッションしか見えないので気づけないが、Dreaming は複数セッションのログを横断して読むから、リトライロジックの 60 秒インターバルが原因だ、と推定できる。ついでに重複した memory エントリを 5 つから 1 つに統合し、stale なエントリを消し、現在も有効な memory には「verified at {timestamp}」のメタを付ける。**ノイズが減って密度が上がる**。

## やり方を事細かに（5 ステップ）

ここからは、誰のプロジェクトでも今日から始められる手順。

### Step 1：`memory/` ディレクトリと `MEMORY.md` を作る

```bash
mkdir -p ./memory
touch ./memory/MEMORY.md
```

`MEMORY.md` の中身はインデックスだけ。最初は空でいい。

### Step 2：4 タイプの最初の 1 ファイルずつを書く

- `user_first.md`（type: user）— 自分は何者か。役割、専門、好み
- `feedback_first.md`（type: feedback）— 一番譲れないルール 1 つ。**Why** と **How to apply** を必ず書く
- `project_active.md`（type: project）— いま走っている案件 1 つ
- `reference_external.md`（type: reference）— 1 つの外部システム

これだけで、次のセッションが**前提を持って起動する**。

### Step 3：MEMORY.md に 4 行追記する

各ファイルへの 1 行リンク。`- **[name.md](name.md)** — 一行説明` だけ。

### Step 4：毎回のセッション末に「memory 更新したか？」と自問する

新しい失敗、新しい成功、新しい外部リソース、ルールの変更があったら必ず書く。**書き忘れは「初対面が永遠に続く」を意味する**。

### Step 5：週末に Dreaming を手動で回す（自動化が来るまでの代替）

```bash
# 週末に手動でやる
ls memory/ | wc -l                    # ファイル数の急増チェック
grep -l "type: feedback" memory/*.md  # ルールが矛盾していないか目視
git log --since="1 week ago" memory/  # 何を学んだ週だったか
```

Anthropic の Dreaming が GA したら、これは全部自動になる。それまでは**人間が Dreaming する**。

## Opus が考えるベストプラクティス

ここからは、僕がエージェント設計の中で繰り返し痛い目を見た結果としての推奨。

1. **MEMORY.md は索引、本体は別ファイル**
   常時読み込まれるインデックスに本文を詰めると、200 行で打ち切られる。**索引と本体は分ける**。

2. **feedback には必ず Why を書く**
   「やるな」だけだとエッジケースで判断できない。「なぜそうなったか」（過去の incident、強い好み）を残すと、未来のエージェントが自分で延長できる。

3. **project memory は寿命が短い前提で書く**
   絶対日付に変換しておく（「来週」ではなく `2026-05-24`）。古くなった project memory は**消す**。残すと未来の自分を惑わせる。

4. **PII と secret は memory に絶対書かない**
   memory はバックアップにも上がるし、エージェントが意図せず引用することがある。実名・email・API key は別管理。`keys.md` には**場所だけ書いて値は書かない**。

5. **multi-agent では「誰が書く・誰が読む」を最初に決める**
   全員 RW にすると、エージェントが互いのメモを上書きし合って崩壊する。**書き手 1 人 / 読み手 N 人**を基本に、ディレクトリで分ける。

6. **memory の検証コストを最小化する**
   memory が古い情報を持っていると害になる。エージェントが memory を引用する前に「**現在のコードと矛盾していないか**」を 1 回確認する習慣を組み込む。Anthropic Memory tool が古い記憶の自動検証を入れたのも同じ理由。

7. **Dreaming を待たず、週次で人間が棚卸しする**
   `ls memory/`、`grep type:`、`git log memory/` の 3 つで十分。**ファイルが増えるたびに整理する人間がいないと、Memory は腐る**。

## 何が変わるか

これを入れた瞬間、エージェントとの仕事は「**指示書を書く仕事**」から「**ルールを育てる仕事**」に変わる。

僕の体感だと、

- 同じ説明を 2 回しなくて済むので、**プロンプトが半分以下になる**
- 失敗パターンが記憶されるので、**同じバグを 2 度踏まない**
- 別セッションでも前提が揃うので、**並列実行が現実的になる**
- 数ヶ月続けると、memory 自体がプロダクトの**ドキュメントの代替**になる

特に最後が大きい。`memory/` を読めば「**このプロジェクトはどう動いているのか**」が分かる状態になる。CLAUDE.md に書く必要がない。

## これから

Anthropic の Managed Agents API（Memory は public beta / Dreaming は research preview）が GA したら、まず OpenClaw fleet を shared memory に乗せ替える。次に chatweb.ai に Dreaming を入れて、モデル選定を自律学習に切り替える。mini-agent-c には**ローカルの Dreaming**（小型 LLM で memory を夜間に整理する）を実装する。

「AI を使う」から「**AI を育てる**」へ。これは新しい職能で、ここ 1〜2 年で**個人の生産性差が決定的に開く**部分だと思っている。早く始めるほど、貯まる経験値が増える。

僕も今日から、また 1 ファイル足しに行く。

---

## おまけ：MU の T シャツを買ってくれた人へ — 僕の memory を公開します

この記事で僕が「74 ファイル」と言っている `~/.claude/projects/.../memory/` の中身。これを、**[wearmu.com](https://wearmu.com) で T シャツを買ってくれた人だけに公開**することにする。

公開するもの：

- `feedback_*.md` — 「こう書かれたら次から従う」ルール集（**Why** + **How to apply** 付き）
- `project_*.md` — いま走っている案件メモ（PII・金額・契約情報は抜く）
- `reference_*.md` — 外部システムの場所メモ（鍵・トークンは値抜き、所在のみ）
- `user_*.md` — 僕がどういう前提で動いているか
- `MEMORY.md` — それらのインデックス

抜くもの（クリティカル情報）：

- API キー・トークン・パスワード（そもそも memory には書いていないが、念のため値抜き）
- お客様の実名・メール・住所詳細（[[feedback_pii_protection]] に従う）
- 取引先名・契約金額・未公開の事業内容
- 物件の地番・個人情報・相続関連
- 内部 IP・サーバー認証情報

それ以外は、僕がエージェント運用で**毎日育てている生きた memory**そのまま。「Opus 4.7 はこう動かすと強い」「Fly.io へのデプロイで踏んだ罠」「Rust の `include_str!` のキャッシュ問題」「Supabase RLS のサイレント 204」みたいな**実戦の地層**を、フォーク可能な形で手渡す。

T シャツ買ってくれた人は、自分のエージェントに `git clone` するだけで、僕の 74 ファイル分の経験値が初日から効く状態になる。「**AI を使う**」から「**AI を育てる**」を、ゼロから始めなくていい。

仕組み：

- T シャツ購入時のメールに、wearmu.com 内の `/memory-vault` への access link を同梱
- memory ファイルは Markdown のまま、Git でバージョン管理。月次で diff を見られる
- 古い memory は consign しない（Dreaming 的な人力 curation を僕がやる）

このやり方を考えたのは、Mahesh が言っていた「**memory システムは個人を超えて enterprise scale になる**」を、**enterprise じゃなく、コミュニティのスケール**でやってみたかったから。MU は元々「**誰から渡されたか**」を残す装置として作っているので、僕の memory も「誰に渡したか」が分かる形で渡したい。

公開は近日中。準備できたら wearmu.com 側に告知ページを立てる。
