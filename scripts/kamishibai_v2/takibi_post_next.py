#!/usr/bin/env python3
"""紙芝居v2の薪キューから「次の1話」をtakibi.wtf焚き火にくべる(1日1話のドリップ配信用)。

- キュー: 同ディレクトリ takibi_queue.json (posted_atで進行管理・冪等)
- 認証: ~/.claude.json の mcpServers.atsm.headers.Authorization を使用(値は表示しない)
- 全話投稿済みなら 'DONE_ALL' を出力して終了(cron側の停止判定に使う)
- ⚠ User-Agent必須(デフォルトUAはbot対策で403)
"""
import json
import os
import subprocess
import sys
from datetime import datetime, timezone, timedelta

HERE = os.path.dirname(os.path.abspath(__file__))
QUEUE = os.path.join(HERE, "takibi_queue.json")
JST = timezone(timedelta(hours=9))

q = json.load(open(QUEUE))
nxt = next((e for e in q["queue"] if not e["posted_at"]), None)
if nxt is None:
    print("DONE_ALL")
    sys.exit(0)

auth = json.load(open(os.path.expanduser("~/.claude.json")))["mcpServers"]["atsm"]["headers"]["Authorization"]
body = {
    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
    "params": {"name": "atsm_log",
               "arguments": {"text": nxt["text"], "member_name": q["member_name"]}},
}
res = subprocess.run(
    ["curl", "-s", "-X", "POST", "https://takibi.wtf/mcp",
     "-H", f"Authorization: {auth}",
     "-H", "Content-Type: application/json",
     "-H", "Accept: application/json",
     "-H", "User-Agent: kamishibai-takibi/1.0",
     "-d", json.dumps(body, ensure_ascii=False)],
    capture_output=True, text=True, timeout=60)
out = res.stdout.strip()
try:
    parsed = json.loads(out)
except Exception:
    print("POST_FAILED raw:", out[:300]); sys.exit(1)
if "error" in parsed or parsed.get("result", {}).get("isError"):
    print("POST_FAILED:", json.dumps(parsed, ensure_ascii=False)[:400]); sys.exit(1)

nxt["posted_at"] = datetime.now(JST).isoformat()
json.dump(q, open(QUEUE, "w"), ensure_ascii=False, indent=1)
remaining = sum(1 for e in q["queue"] if not e["posted_at"])
print(f"POSTED {nxt['ep']} / remaining {remaining}")
print("server_said:", json.dumps(parsed.get("result", {}), ensure_ascii=False)[:300])
