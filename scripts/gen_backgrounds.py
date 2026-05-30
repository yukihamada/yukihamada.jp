#!/usr/bin/env python3
"""Generate cinematic dark-mode background photos for yukihamada.jp slideshow.
Model: gemini-3-pro-image-preview (per global SOP). Outputs to public/assets/photos/.
"""
import os, sys, pathlib
from google import genai
from google.genai import types

OUT = pathlib.Path(__file__).resolve().parent.parent / "public" / "assets" / "photos"
OUT.mkdir(parents=True, exist_ok=True)

key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
client = genai.Client(api_key=key)

STYLE = ("Cinematic, moody, dark low-key photography, deep shadows, subtle film grain, "
         "wide 16:9 landscape, no people, no text, no logos, high detail, atmospheric. ")

JOBS = {
    "bg-aurora-teshikaga": STYLE + "Aurora and northern lights over a frozen lake in the Hokkaido wilderness near Teshikaga, faint green and violet glow, silhouetted spruce forest, snow.",
    "bg-misty-forest": STYLE + "Misty old-growth Hokkaido forest at dawn, fog drifting between tall trees, faint warm light breaking through, damp earth.",
    "bg-ocean-night": STYLE + "Dark Pacific ocean at night under a vast starfield, gentle long-exposure waves, faint horizon glow, deep blue and black tones.",
    "bg-snow-cabin": STYLE + "A small modern wooden cabin glowing warm light from within, surrounded by deep snow at blue hour, mountains behind, isolated and serene.",
    "bg-mountain-dusk": STYLE + "Layered mountain ridges fading into dusk haze, gradient of indigo to deep orange sky, single faint star, minimalist and vast.",
    "bg-tatami-light": STYLE + "Quiet empty dojo interior at night, single shaft of moonlight across the mat, dust in the air, calm and contemplative, minimal.",
}

def gen(name, prompt):
    resp = client.models.generate_content(
        model="gemini-3-pro-image-preview",
        contents=prompt,
        config=types.GenerateContentConfig(response_modalities=["IMAGE", "TEXT"]),
    )
    for part in resp.candidates[0].content.parts:
        if getattr(part, "inline_data", None) and part.inline_data.data:
            p = OUT / f"{name}.jpg"
            p.write_bytes(part.inline_data.data)
            print(f"OK  {p}  ({len(part.inline_data.data)//1024} KB)")
            return True
    print(f"ERR {name}: no image returned")
    return False

if __name__ == "__main__":
    only = sys.argv[1:] or list(JOBS.keys())
    ok = 0
    for n in only:
        try:
            if gen(n, JOBS[n]):
                ok += 1
        except Exception as e:
            print(f"ERR {n}: {e}")
    print(f"\n{ok}/{len(only)} generated")
