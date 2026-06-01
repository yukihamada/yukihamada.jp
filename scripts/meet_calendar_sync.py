#!/usr/bin/env python3
"""Continuous freebusy sync for yukihamada.jp/meet.

Runs locally (where `gog` is authenticated). Fetches the candidate slots from
the site, checks each against Yuki's real Google Calendar via `gog`, and POSTs
the busy slot-ids back. The site hides busy slots (fail-open if this stops).

Env:
  MEET_ADMIN_TOKEN  required — admin token (from ~/.cron_secrets/meet_sync.env)
  MEET_BASE         optional — default https://yukihamada.jp
  GOG_ACCOUNT       optional — default yuki@hamada.tokyo
"""
import os, sys, json, subprocess, datetime, urllib.request

BASE = os.environ.get("MEET_BASE", "https://yukihamada.jp").rstrip("/")
TOKEN = os.environ.get("MEET_ADMIN_TOKEN", "")
ACCOUNT = os.environ.get("GOG_ACCOUNT", "yuki@hamada.tokyo")
GOG = os.environ.get("GOG_BIN", "/opt/homebrew/bin/gog")
JST = datetime.timezone(datetime.timedelta(hours=9))

if not TOKEN:
    print("MEET_ADMIN_TOKEN not set", file=sys.stderr); sys.exit(2)


def http_json(url, data=None):
    body = json.dumps(data).encode() if data is not None else None
    req = urllib.request.Request(
        url, data=body, headers={"content-type": "application/json"},
        method="POST" if data is not None else "GET")
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.loads(r.read().decode())


def to_epoch(s):
    if not s:
        return None
    if isinstance(s, dict):
        s = s.get("dateTime") or s.get("date")
    if not s:
        return None
    s = s.replace("Z", "+00:00")
    try:
        dt = datetime.datetime.fromisoformat(s)
    except ValueError:
        try:
            dt = datetime.datetime.fromisoformat(s + "T00:00:00+09:00")
        except ValueError:
            return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=JST)
    return dt.timestamp()


def main():
    # 1) candidate slots from the live site
    listing = http_json(f"{BASE}/api/meet/admin/list?token={TOKEN}")
    slots = listing.get("all_slots", [])
    if not slots:
        print("no candidate slots"); return

    # 2) window covering all slots
    starts = []
    for sid in slots:
        try:
            starts.append(datetime.datetime.fromisoformat(sid).replace(tzinfo=JST))
        except ValueError:
            pass
    if not starts:
        return
    frm = (min(starts) - datetime.timedelta(days=1)).date().isoformat()
    to = (max(starts) + datetime.timedelta(days=2)).date().isoformat()

    # 3) real calendar events via gog
    out = subprocess.run(
        [GOG, "calendar", "events", "--account", ACCOUNT, "--all",
         "--from", frm, "--to", to, "--max", "500", "--json"],
        capture_output=True, text=True, timeout=60)
    raw = (out.stdout or "").strip()
    events = []
    if raw:
        data = json.loads(raw)
        events = data if isinstance(data, list) else data.get("events", data.get("items", []))

    busy_intervals = []
    for e in events:
        if not isinstance(e, dict):
            continue
        a = to_epoch(e.get("start") or e.get("start_time") or e.get("startTime"))
        b = to_epoch(e.get("end") or e.get("end_time") or e.get("endTime"))
        if a and b:
            busy_intervals.append((a, b))

    # 4) which candidate slots clash (1h each)
    busy_slots = []
    for sid in slots:
        try:
            st = datetime.datetime.fromisoformat(sid).replace(tzinfo=JST).timestamp()
        except ValueError:
            continue
        en = st + 3600
        if any(st < b and a < en for a, b in busy_intervals):
            busy_slots.append(sid)

    # 5) push
    res = http_json(f"{BASE}/api/meet/admin/busy", {"token": TOKEN, "slots": busy_slots})
    print(f"events={len(busy_intervals)} candidate={len(slots)} busy={busy_slots} -> {res}")


if __name__ == "__main__":
    main()
