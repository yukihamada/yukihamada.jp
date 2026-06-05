#!/usr/bin/env python3
"""Generate per-scene cloned-voice MP3s for the Give & Take kamishibai (EP8).
Reads scripts/give_scenes.json -> public/audio/give-kam-{1..9}.mp3
Usage: python3 scripts/gen_give_audio.py [scene_numbers...]
"""
import os, sys, json, time, urllib.request, urllib.error

KEY = os.environ.get("ELEVENLABS_API_KEY", "")
VOICE_ID = "VneiyrGsB8R1ym9S1XYl"   # Yuki Hamada cloned voice
MODEL_ID = "eleven_multilingual_v2"
SETTINGS = {"stability": 0.62, "similarity_boost": 0.80, "style": 0.15, "use_speaker_boost": True}

HERE = os.path.dirname(__file__)
SCENES = json.load(open(os.path.join(HERE, "give_scenes.json"), encoding="utf-8"))
OUT = os.path.join(HERE, "..", "public", "audio")

def tts(i, text):
    url = f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE_ID}"
    body = json.dumps({"text": text, "model_id": MODEL_ID, "voice_settings": SETTINGS}).encode()
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, data=body, headers={
                "xi-api-key": KEY, "Content-Type": "application/json", "Accept": "audio/mpeg"})
            with urllib.request.urlopen(req, timeout=60) as r:
                audio = r.read()
            out = os.path.join(OUT, f"give-kam-{i}.mp3")
            with open(out, "wb") as f:
                f.write(audio)
            print(f"  give-kam-{i}.mp3  {len(audio)//1024}KB  OK")
            return True
        except urllib.error.HTTPError as e:
            print(f"  scene{i}: HTTP {e.code} {e.read()[:200]} (attempt {attempt+1})")
            time.sleep(2 ** attempt)
        except Exception as e:
            print(f"  scene{i}: {e} (attempt {attempt+1})")
            time.sleep(2)
    return False

def main():
    if not KEY:
        sys.exit("no ELEVENLABS_API_KEY")
    only = [int(x) for x in sys.argv[1:]] if len(sys.argv) > 1 else list(range(1, 10))
    fail = []
    for i in only:
        text = SCENES[i-1].get("tts") or SCENES[i-1]["text"]
        print(f"[{i}] {len(text)}chars: {text[:30]}...")
        if not tts(i, text):
            fail.append(i)
        time.sleep(0.4)
    print("DONE. failed:", fail if fail else "none")
    sys.exit(1 if fail else 0)

if __name__ == "__main__":
    main()
