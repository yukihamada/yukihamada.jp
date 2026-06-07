#!/usr/bin/env python3
"""kamishibai v2 音声生成 — ElevenLabsベストプラクティス実装版

調査済みベストプラクティス(2026-06):
  1. eleven_v3 を第一候補: Audio Tags([whispers]/[sighs]/[rushed]等)+三点リーダ/ダッシュで
     感情と間を制御。SSML <break> は v3 非対応。
  2. eleven_multilingual_v2 をフォールバック: 「……」→ <break time="0.7s"/> 変換。
     break多用は不安定化するため1文に2個まで。
  3. Request Stitching: previous_text / next_text で前後シーンの韻律を接続
     (シーン分割生成のつなぎ目の不自然さを解消)。
  4. voice_settings: storytelling は stability 0.40-0.55 / similarity <=0.80 /
     style 0.03-0.08 / speaker_boost(v2のみ)。旧設定(stability0.62/style0.15)は硬すぎ。
  5. v3 は世代間ばらつきがある → --takes N でBest-of-N生成し、mlx_whisper の
     転写一致率で自動選抜。

使い方:
  export ELEVENLABS_API_KEY=...
  python3 scripts/gen_kamishibai_v2_audio.py EP4            # v3で全シーン(2テイク選抜)
  python3 scripts/gen_kamishibai_v2_audio.py EP4 --model v2 # v2フォールバック
  python3 scripts/gen_kamishibai_v2_audio.py all --takes 3
出力: public/audio/<prefix>-kam-N.mp3 (既存命名規則を踏襲)
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.request

VOICE_ID = "VneiyrGsB8R1ym9S1XYl"  # 濱田優貴クローン
API_KEY = os.environ.get("ELEVENLABS_API_KEY", "")
BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
V2DIR = os.path.join(BASE, "scripts", "kamishibai_v2")
OUTDIR = os.path.join(BASE, "public", "audio", "v2")  # 既存本番mp3を上書きしない分離ディレクトリ
os.makedirs(OUTDIR, exist_ok=True)

# 既存mp3命名規則(HTMLテンプレが参照)
PREFIX = {
    "EP1": "ep1v2", "EP2": "ep2v2",  # EP1/2は元が一本mp3。v2はシーン分割の新規プレフィックス
    "EP3": "asoview", "EP4": "atsume", "EP5": "kagi", "EP6": "minna",
    "EP7": "transformer", "EP8": "give", "magmag": "mg-14b7a2be",
    "BLANK": "blank",
}

TAG_RE = re.compile(r"\[[a-z ]+\]")


def strip_tags(text: str) -> str:
    """Audio Tags除去(whisper照合・stitching用の素テキスト)"""
    return TAG_RE.sub("", text).replace("……", "").replace("——", "").strip()


def v2_breaks(text: str) -> str:
    """v2用: 「……」→ break tag。1文2個までに制限(安定性ガイドライン)"""
    out, count = [], 0
    for chunk in text.split("。"):
        c = chunk
        n = c.count("……")
        if n > 2:  # 過剰なbreakは不安定化 → 3個目以降は読点に落とす
            c = c.replace("……", "、", n - 2)
        c = c.replace("……", ' <break time="0.7s" /> ')
        out.append(c)
    return "。".join(out)


def tts_request(text: str, model: str, prev: str, nxt: str, speed: float = 1.0) -> bytes:
    url = f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE_ID}?output_format=mp3_44100_128"
    if model == "eleven_v3":
        settings = {"stability": 0.5, "similarity_boost": 0.75}  # Natural。speaker_boostはv3非対応
    else:
        settings = {"stability": 0.48, "similarity_boost": 0.78,
                    "style": 0.05, "use_speaker_boost": True}
    if speed != 1.0:
        settings["speed"] = speed  # 0.7-1.2。v3も対応確認済み(2026-06-06)
    payload = {"text": text, "model_id": model, "voice_settings": settings}
    if model != "eleven_v3":
        # Request Stitching は v2系のみ対応(v3は unsupported_model で400)
        if prev:
            payload["previous_text"] = prev
        if nxt:
            payload["next_text"] = nxt
    req = urllib.request.Request(
        url, data=json.dumps(payload).encode(),
        headers={"xi-api-key": API_KEY, "Content-Type": "application/json",
                 "Accept": "audio/mpeg"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def whisper_score(mp3_path: str, expected: str) -> float:
    """mlx_whisperで転写し、正規化一致率を返す(Best-of-N選抜用)"""
    import difflib
    with tempfile.TemporaryDirectory() as td:
        try:
            subprocess.run(
                ["mlx_whisper", mp3_path, "--model",
                 "mlx-community/whisper-large-v3-turbo", "--language", "ja",
                 "--output-format", "txt", "--output-dir", td],
                capture_output=True, timeout=300, check=True)
        except Exception:
            return -1.0
        txts = [f for f in os.listdir(td) if f.endswith(".txt")]
        if not txts:
            return -1.0
        got = open(os.path.join(td, txts[0])).read()
    norm = lambda s: re.sub(r"[、。「」…—\s・?!?!]", "", s)
    return difflib.SequenceMatcher(None, norm(expected), norm(got)).ratio()


def gen_episode(ep_id: str, model_key: str, takes: int, speed: float = 1.0):
    data = json.load(open(os.path.join(V2DIR, f"{ep_id}.json")))
    scenes = data["scenes"]
    model = "eleven_v3" if model_key == "v3" else "eleven_multilingual_v2"
    prefix = PREFIX[ep_id]
    print(f"== {ep_id} {data['title']} / {model} / takes={takes} / speed={speed}")
    for i, sc in enumerate(scenes):
        raw = sc.get("tts_v3") if model_key == "v3" else sc.get("tts")
        text = raw if model_key == "v3" else v2_breaks(raw)
        prev = strip_tags(scenes[i - 1].get("tts", ""))[-280:] if i > 0 else ""
        nxt = strip_tags(scenes[i + 1].get("tts", ""))[:280] if i < len(scenes) - 1 else ""
        expected = strip_tags(sc.get("tts", ""))
        out = os.path.join(OUTDIR, f"{prefix}-kam-{sc['n']}.mp3")
        if os.path.exists(out) and os.path.getsize(out) > 10000:
            print(f"  scene{sc['n']}: skip (既存mp3あり — 再生成は先にmp3を削除)")
            continue
        best, best_score = None, -2.0
        for t in range(takes):
            try:
                audio = tts_request(text, model, prev, nxt, speed)
            except Exception as e:
                print(f"  scene{sc['n']} take{t+1}: ERROR {e}")
                time.sleep(3)
                continue
            score = whisper_score_bytes(audio, expected) if takes > 1 else 1.0
            print(f"  scene{sc['n']} take{t+1}: {len(audio)//1024}KB score={score:.3f}")
            if score > best_score:
                best, best_score = audio, score
            time.sleep(1)
        if best is None:
            print(f"  scene{sc['n']}: FAILED all takes — 既存mp3を保持")
            continue
        open(out, "wb").write(best)
        print(f"  -> {out} (score={best_score:.3f})")


def whisper_score_bytes(audio: bytes, expected: str) -> float:
    with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as f:
        f.write(audio)
        path = f.name
    try:
        return whisper_score(path, expected)
    finally:
        os.unlink(path)


if __name__ == "__main__":
    if not API_KEY:
        sys.exit("ELEVENLABS_API_KEY が未設定です")
    args = sys.argv[1:]
    target = args[0] if args else "all"
    model_key = "v2" if "--model" in args and "v2" in args else "v3"
    takes = int(args[args.index("--takes") + 1]) if "--takes" in args else 2
    speed = float(args[args.index("--speed") + 1]) if "--speed" in args else 1.0
    eps = list(PREFIX.keys()) if target == "all" else [target]
    for ep in eps:
        gen_episode(ep, model_key, takes, speed)
