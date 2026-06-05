#!/usr/bin/env python3
"""Generate 9 sumi-e kamishibai scene images for the Give & Take episode (EP8)
via Gemini gemini-3-pro-image-preview. Style matches EP7 (transformer-kamishibai).
Output: public/assets/give-kamishibai/scene1.png .. scene9.png
"""
import os, sys, json, base64, time, urllib.request, urllib.error

KEY = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
MODEL = "gemini-3-pro-image-preview"
OUT = os.path.join(os.path.dirname(__file__), "..", "public", "assets", "give-kamishibai")
os.makedirs(OUT, exist_ok=True)

STYLE = ("Japanese sumi-e ink-wash painting, traditional brush strokes, deep charcoal-black "
         "background, vast dark negative space, a single warm amber glow as the only light source, "
         "soft atmospheric mist, cinematic, minimal, painterly. A small lone figure in a simple robe, "
         "seen from behind, stands at the lower-center for scale. Absolutely NO text, NO letters, "
         "NO words, NO numbers, NO captions, NO symbols anywhere in the image. "
         "Wide 16:9 cinematic composition. The focal motif sits in the upper-center; "
         "keep the bottom third dark and simple for a subtitle overlay.")

SCENES = [
 "The figure holds out both open hands, offering a single small warm amber ember up into the vast darkness; faint dim figures stand far around in the gloom. A quiet gesture of giving into the void.",
 "Three distinct glowing motifs arranged as an elegant trio in the dark: on the left a pair of open hands pouring warm light outward (the giver), in the center two grasping hands pulling light inward toward themselves (the taker), on the right a perfectly level balance scale of light (the matcher). The figure stands before the trio.",
 "A serene smiling glowing mask floats in front, but behind it a single dim grasping hand reaches out to take; a sharp amber highlight reveals the hidden hand. Two-faced, a quiet warning.",
 "A bright giving figure whose warm amber light is being siphoned away through many dark grasping threads pulling outward to grabbing hands at the edges; the figure grows thin and dim, drained. A sense of being consumed.",
 "The very same open giving hands, now risen to the summit of a dark peak, radiating a tall brilliant column of warm amber light upward into the sky — the one who gives and yet stands tallest. Triumphant but warm.",
 "Two hands and a balance scale of light between them: one hand gives a warm orb, and only when an orb is placed back on the empty pan does the scale glow level — reciprocity. A luminous thread connects give and return.",
 "A wide night desert under stars, a great ring of small glowing lanterns and tents encircling one central warm bonfire; small figures pass glowing gifts hand to hand around the circle. No buying, only giving.",
 "Many hands from all directions placing small logs and twigs onto a single growing campfire; the warm amber flame leaps tall and bright, golden sparks rising into the black sky. Shared fire.",
 "The lone figure kneels and places the first single small log onto a young fire; from the ember a warm amber light spreads outward across the ground toward a distant horizon where a calm sea meets the sky. Hopeful finale.",
]

def gen(i, motif):
    prompt = STYLE + "\n\nScene: " + motif
    body = json.dumps({
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {"responseModalities": ["IMAGE", "TEXT"],
                              "imageConfig": {"aspectRatio": "16:9"}},
    }).encode()
    url = (f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={KEY}")
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=180) as r:
                data = json.loads(r.read())
            parts = data["candidates"][0]["content"]["parts"]
            for p in parts:
                blob = p.get("inlineData") or p.get("inline_data")
                if blob and blob.get("data"):
                    out = os.path.join(OUT, f"scene{i}.png")
                    with open(out, "wb") as f:
                        f.write(base64.b64decode(blob["data"]))
                    print(f"  scene{i}.png  {os.path.getsize(out)//1024}KB  OK")
                    return True
            print(f"  scene{i}: no image part (attempt {attempt+1}); resp keys={list(data.get('candidates',[{}])[0].get('content',{}).keys())}")
        except urllib.error.HTTPError as e:
            print(f"  scene{i}: HTTP {e.code} {e.read()[:200]} (attempt {attempt+1})")
        except Exception as e:
            print(f"  scene{i}: {e} (attempt {attempt+1})")
        time.sleep(3 + attempt * 3)
    return False

def main():
    if not KEY:
        sys.exit("no GEMINI_API_KEY")
    only = [int(x) for x in sys.argv[1:]] if len(sys.argv) > 1 else list(range(1, 10))
    fail = []
    for i in only:
        print(f"[{i}/9] generating...")
        if not gen(i, SCENES[i-1]):
            fail.append(i)
        time.sleep(1)
    print("DONE. failed:", fail if fail else "none")
    sys.exit(1 if fail else 0)

if __name__ == "__main__":
    main()
