#!/bin/zsh
# 紙芝居v2 薪ドリップ配信 (launchd: com.yuki.kamishibai-takibi が毎日21:00 JSTに実行)
# POSTED=成功 / POST_FAILED=120秒後に1回だけリトライ / DONE_ALL=全話完了→ジョブ自身を解除
set -u
SCRIPT="/Users/yuki/workspace/yukihamada.jp/scripts/kamishibai_v2/takibi_post_next.py"
PLIST="$HOME/Library/LaunchAgents/com.yuki.kamishibai-takibi.plist"
LABEL="com.yuki.kamishibai-takibi"

out=$(/usr/bin/python3 "$SCRIPT" 2>&1)
echo "$(date '+%Y-%m-%d %H:%M:%S') $out"

case "$out" in
  *POST_FAILED*)
    sleep 120
    out2=$(/usr/bin/python3 "$SCRIPT" 2>&1)
    echo "$(date '+%Y-%m-%d %H:%M:%S') retry: $out2"
    ;;
  *DONE_ALL*)
    echo "$(date '+%Y-%m-%d %H:%M:%S') 全8話配信完了。ジョブを解除します"
    /bin/launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
    rm -f "$PLIST"
    ;;
esac
