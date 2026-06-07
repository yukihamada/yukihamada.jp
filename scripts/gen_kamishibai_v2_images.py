#!/usr/bin/env python3
"""kamishibai v2 シーン画像生成 — 各話JSONのvisual指示からGeminiで生成

- モデル: gemini-3-pro-image-preview (固定・CLAUDE.md指定)
- 出力: public/assets/kamishibai-v2/<EP>/scene<n>.png (16:9)
- 文字は画像に入れない(言語非依存の視覚文法・v2設計の核)
- magmagは対象外(NDA・private overlay側で扱う)
"""
import base64
import json
import os
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor

KEY = os.environ.get("GEMINI_API_KEY", "")
MODEL = "gemini-3-pro-image-preview"
BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EPS = sys.argv[1:] or ["EP1", "EP2", "EP3", "EP4", "EP5", "EP6", "EP7", "EP8"]

STYLE = (
    "Japanese modern ink-art (sumi-e meets cinematic) keyframe for a digital kamishibai. "
    "Palette: deep sumi black #1a1714 base, vermilion #c0392b accents, warm candle light #f5d9a8, ember orange #e6953f. "
    "16:9 cinematic composition, strong use of negative space, film grain, soft vignette. "
    "ABSOLUTELY NO text, NO letters, NO numbers, NO captions inside the image. "
    "Consistent character: a simple silhouetted figure (no facial detail). Mood follows the scene direction."
)

# BLANK編は反転パレット(紙白基調・霧)。視覚文法はv2共通(文字ゼロ・シルエット)
STYLE_OVERRIDE = {
    "BLANK": (
        "Japanese modern ink-art (sumi-e meets cinematic) keyframe for a digital kamishibai, INVERTED palette. "
        "Palette: paper-white #f5f3ef base, morning fog grey #d9d6d0, sumi black #1a1714 silhouettes and linework, "
        "vermilion #c0392b accents, ember orange #e6953f for fire, warm light #f5d9a8, lake blue #3a6ea5 hints. "
        "16:9 cinematic composition, vast negative space of white fog (Lake Mashu, Hokkaido), film grain, soft vignette. "
        "ABSOLUTELY NO text, NO letters, NO numbers, NO captions inside the image. "
        "Consistent characters: simple silhouetted figures (no facial detail). Mood follows the scene direction."
    ),
    # RECAP編(ふたつの白=熱海・水上振り返り)もBLANKと同じ反転白パレット。海/湯気/森のモチーフ
    "RECAP": (
        "Japanese modern ink-art (sumi-e meets cinematic) keyframe for a digital kamishibai, INVERTED palette. "
        "Palette: paper-white #f5f3ef base, morning fog grey #d9d6d0, sumi black #1a1714 silhouettes and linework, "
        "vermilion #c0392b accents, cream #e9e4d8 soft glow, warm light #f5d9a8, sea/lake blue #3a6ea5 hints. "
        "16:9 cinematic composition, vast negative space of white steam and fog (onsen town by the sea, forest hot-spring), "
        "film grain, soft vignette. "
        "ABSOLUTELY NO text, NO letters, NO numbers, NO captions inside the image. "
        "Consistent characters: simple silhouetted figures (no facial detail). Mood follows the scene direction."
    ),
}


def gen_image(prompt: str, out_path: str, retries: int = 3) -> bool:
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={KEY}"
    body = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {"responseModalities": ["IMAGE", "TEXT"]},
    }
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, data=json.dumps(body).encode(),
                                         headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=120) as r:
                res = json.loads(r.read())
            for part in res.get("candidates", [{}])[0].get("content", {}).get("parts", []):
                if "inlineData" in part:
                    open(out_path, "wb").write(base64.b64decode(part["inlineData"]["data"]))
                    return True
            time.sleep(2)
        except Exception as e:
            print(f"  retry{attempt+1}: {type(e).__name__} {str(e)[:80]}", flush=True)
            time.sleep(5 * (attempt + 1))
    return False


def scene_jobs():
    for ep in EPS:
        d = json.load(open(os.path.join(BASE, "scripts", "kamishibai_v2", f"{ep}.json")))
        outdir = os.path.join(BASE, "public", "assets", "kamishibai-v2", ep)
        os.makedirs(outdir, exist_ok=True)
        for sc in d["scenes"]:
            out = os.path.join(outdir, f"scene{sc['n']}.png")
            if os.path.exists(out) and os.path.getsize(out) > 30000:
                continue  # レジューム可
            prompt = (
                f"{STYLE_OVERRIDE.get(ep, STYLE)}\n\nEpisode theme: {d['logline']}\n"
                f"Scene direction (Japanese, follow precisely): {sc['visual'][:600]}\n"
                f"Emotional tone of narration: {sc['narration'][:160]}"
            )
            yield (ep, sc["n"], prompt, out)


def run(job):
    ep, n, prompt, out = job
    ok = gen_image(prompt, out)
    size = os.path.getsize(out) // 1024 if ok and os.path.exists(out) else 0
    print(f"{'OK ' if ok else 'NG '}{ep} scene{n} ({size}KB)", flush=True)
    return ok


if __name__ == "__main__":
    if not KEY:
        sys.exit("GEMINI_API_KEY 未設定")
    jobs = list(scene_jobs())
    print(f"生成対象: {len(jobs)}枚", flush=True)
    with ThreadPoolExecutor(max_workers=4) as ex:
        results = list(ex.map(run, jobs))
    print(f"完了: {sum(results)}/{len(jobs)}", flush=True)
