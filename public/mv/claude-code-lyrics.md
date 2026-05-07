# claude code （クロードコード）
## — 開発者のための実践ガイドソング

### Intro (0-12s)
```
ターミナルを開いて claude と打つ
世界が変わる たった6文字で
```

### Verse 1: セットアップ (12-36s)
```
まずCLAUDE.mdを書け ルールブック
プロジェクトの掟 AIへの指示書
「Rustで書け」「Fly.ioにデプロイ」「テスト必須」
一度書けば 何度でも従う

.claude/settings.json パーミッション
allowedTools に Bash Edit Read
毎回の承認が面倒なら
ここに書け ワンタイムじゃない

探索 計画 実装 この順番
いきなりコード書くな まずgrepしろ
tasks/todo.md に設計図を描け
シニアエンジニアなら当然だろ？
```

### Chorus 1 (36-52s)
```
claude code 走らせろ
Edit Read Grep Glob 使い分けろ
サブエージェント並列で飛ばせ
一人で開発してるのに チーム以上

claude code 止まるな
fly deploy --remote-only
エラーが出たら lessons.md に刻め
同じミス 二度としない それがルール
```

### Verse 2: 実践テクニック (52-80s)
```
Read で先にファイルを読め
中身を知らずに Edit するな
old_string は一意に 長めに取れ
replace_all は変数リネームの時だけ

Grep でコード検索 rg じゃなく
Glob でファイル探し find じゃなく
Bash は最後の手段 専用ツールが先
これだけで作業速度が倍になる

サブエージェント 使い所を見極めろ
Explore は深い調査 3クエリ以上の時
fly-deployer ts-builder rust-lambda
適材適所 並列で走らせろ
```

### Chorus 2 (80-96s)
```
claude code 走らせろ
メモリに保存 次の会話で思い出す
feedback user project reference
四種のメモリ 使い分けろ

claude code 止まるな
/commit でコミット /review でレビュー
Skill を叩け スラッシュコマンド
ワンライナーで 世界をデプロイ
```

### Bridge: デバッグの極意 (96-120s)
```
エラーが出た 推測で直すな
スタックトレース 読め 原因を追え
3回行き詰まったら 立ち止まれ
人間に聞け それも立派なスキル

DynamoDB の env 全置換トラップ
--cli-input-json で差分更新
Lambda は musl ビルド gnu は死ぬ
include_str は cargo clean してリビルド

Supabase RLS 204が返っても安心するな
Prefer: return=representation つけろ
見えない壁に何時間も溶かした
lessons.md がなかったら 同じ穴に落ちてた
```

### Verse 3: 応用とフロー (120-144s)
```
Hooks で自動化 設定は settings.json
コミット前にlint テスト後にデプロイ
from now on は Hooks で書け
メモリじゃない ハーネスが実行する

MCP サーバーで外部ツール接続
Puppeteer Telegram Supabase
ブラウザ操作も メッセージ送信も
Claude Code の中で全部完結

Worktree isolation 安全に実験
本体ブランチを汚さず 別世界で試す
Plan mode で設計 Exit で実装
アーキテクトとエンジニア 一人二役
```

### Final Chorus (144-164s)
```
claude code 走らせろ
20以上のプロダクト 一つのワークスペース
Rust Swift React TypeScript
全部の言語を 一人で回す

claude code 未来はここ
コードを書いて テストして デプロイ
人間がレビュー AIが実装
最強のペアプロ それが claude code
```

### Outro (164-180s)
```
ターミナルを閉じるな まだ終わってない
lessons.md を更新しろ
明日の自分への 最高のギフト
claude と打て 開発は続く
```

---

## 実践Tips（歌詞に込めた情報まとめ）

1. **CLAUDE.md**: プロジェクトルートに置く。AIへの恒久的な指示書
2. **settings.json**: パーミッション、Hooks、環境変数の設定
3. **探索→計画→実装**: いきなりコードを書かない
4. **Read→Edit**: 必ず読んでから編集
5. **Grep/Glob > bash grep/find**: 専用ツールを優先
6. **サブエージェント**: 並列タスクは Agent tool で委譲
7. **メモリ4種**: user/feedback/project/reference
8. **lessons.md**: 過去の失敗を記録→再発防止
9. **Hooks**: 自動化は settings.json の hooks で
10. **MCP**: 外部ツール連携（Puppeteer, Telegram等）
11. **Worktree**: isolation モードで安全に実験
12. **Lambda musl**: gnu ではなく musl を使え
13. **DynamoDB env**: --cli-input-json で差分更新
14. **Supabase RLS**: Prefer ヘッダー必須
15. **/commit /review**: Skill（スラッシュコマンド）活用
