---
title: "C言語1500行でClaude Codeクローンを作り、ローカルLLMで動かした"
date: 2026-04-13
description: "自律AIエージェントをC言語で1から実装。ツール実行・プロンプトキャッシュ・サブエージェントまで全部入り。最終的にM5 MacのQwen3.5-122BでAPIコスト$0で動かすところまで。"
tags: ["AI", "C言語", "ローカルLLM", "エージェント", "Qwen", "MLX", "開発記録"]
---

Claude Codeは便利だ。でもブラックボックスだ。

内部でどんなプロンプトが飛んでいるか、ツール実行のループがどう機能しているか、コンテキストがどう管理されているか——全部隠れている。

それなら自分で書こうと思った。**C言語で。**

理由は3つ。

1. **依存ゼロ**にしたかった（libcurl と cJSON だけ）
2. **理解できる**コードにしたかった。Rustなら500行で書けるが可読性が落ちる
3. **移植性**。どのLinux/macOSでも `cc agent.c cJSON.c -lcurl -lm` だけで動く

結果として、v1〜v11まで11バージョン、約3000行のCコードができあがった。

## アーキテクチャ：単純なループ

Claude Codeのコアは単純なループだ。

```
ユーザーのタスク
    ↓
[LLM API呼び出し]
    ↓
tool_use ブロックが返ってきたら
    ↓
ツールを実行（bash / read / write / grep / git...）
    ↓
結果をmessagesに追加してループ
    ↓
stop_reason = "end_turn" になったら終了
```

これを実装したのが v1（10KB、約300行）。

## v1 → v11 の進化

| バージョン | 追加機能 |
|---|---|
| v1 | tool_useループ、bash/read/write、パス制限 |
| v2 | edit_file（精密な文字列置換） |
| v3 | プロンプトキャッシュ、永続メモリ、サブエージェント |
| v4 | planモード、sandboxモード、コスト追跡 |
| v5 | OpenAI互換バックエンド、--api-base |
| v6 | max-turns制限、トークンバジェット |
| v7 | 拡張思考モード、REPL |
| v8 | grep/glob、http_get、todo管理 |
| v9 | 並列ツール実行、ask_user、git、notify |
| v10 | http_request、checkpoint/undo、プロセス管理、クリップボード |
| v11 | **--approve: ツール実行前の人間確認モード** |

## 実際のデモ：バグを自分で見つけて直す

試したタスク:

```
「Cの電卓プログラムを作って、テストスクリプトも書いて、全部通るまで直して」
```

エージェントが自律的にやったこと:

```
turn 1: todo追加 「calc.c実装」
turn 2: write_file → calc.c（浮動小数点対応）
turn 3: write_file → Makefile
turn 4: bash → make（ビルド成功）
turn 5-6: write_file → test_calc.sh、chmod +x
turn 7: テスト実行 → 8/9失敗
turn 8: 「Expected 7, got 7」なのにFAILの原因を特定
        → macOS BSD awk の "exp" が組み込み関数名と衝突
turn 9: edit_file → "exp" を "expected" に修正
turn 10: テスト再実行 → 9/9 PASS
```

APIコスト: 約$0.09。

最初は「Expected 7, got 7なのになぜFAIL？」という謎のバグだった。awk の `-v exp=7` の `exp` が `e^x` の組み込み関数名と衝突していた——macOS BSD awkの罠。これをエージェントが自分で追跡して修正した。

## ローカルLLMで動かす

Anthropic APIへの依存をなくしたかった。

M5 MacBook Pro（128GBメモリ）に**Qwen3.5-122B**をMLX量子化（4bit）で動かしている。構成:

```
mini-agent-c --api-base http://localhost:4001
    ↓
proxy-rs（Rust）← Anthropic形式 → OpenAI形式変換
    ↓
mlx_lm.server :5000 ← Qwen3.5-122B-A10B-4bit
```

`--api-base` フラグを追加する際に1つバグを見つけた。`claude_api_once()` が `https://api.anthropic.com` をハードコードしていてフラグを無視していた。3行で直した:

```c
// 修正前
curl_easy_setopt(curl, CURLOPT_URL, "https://api.anthropic.com/v1/messages");

// 修正後
char url[512];
snprintf(url, sizeof(url), "%s/v1/messages",
    g_api_base[0] ? g_api_base : "https://api.anthropic.com");
curl_easy_setopt(curl, CURLOPT_URL, url);
```

## v11：--approve で人間がループに入る

v11の新機能は `--approve` フラグ。ツールを実行する前に確認を求める:

```
[approve] bash {"command":"rm -rf temp/"}
  [y]es  [n]o/skip  [a]bort  [!]yes-to-all ?
```

- `--approve` : 全ツール確認
- `--approve-bash` : bashコマンドのみ確認（推奨）
- `!` : 残り全部自動実行に切り替え

AIが自律的に動きながら、危険な操作は人間が止められる。

## なぜCなのか

書き終わって思うのは、**Cは制御できる**ということだ。

- どこでメモリを確保して解放するかが分かる
- HTTPリクエストの中身が1行ずつ追える
- tool_useのJSONパースがどこで起きるか分かる

「Claude Codeはどう動いているのか」という疑問の答えが、C言語のファイルに全部入っている。

---

コードは公開している: [github.com/yukihamada/mini-agent-c](https://github.com/yukihamada/mini-agent-c)

```bash
git clone https://github.com/yukihamada/mini-agent-c
cd mini-agent-c
cc -O2 -o agent agent.c cJSON.c -lcurl -lm
ANTHROPIC_API_KEY=sk-xxx ./agent "hello"
```

libcurl と cJSON だけ。それだけで動く。
