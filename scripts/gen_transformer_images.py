#!/usr/bin/env python3
"""Generate 9 sumi-e kamishibai scene images for the Transformer episode (EP7)
via Gemini gemini-3-pro-image-preview. Style matches EP6 (minna-kamishibai).
Output: public/assets/transformer-kamishibai/scene1.png .. scene9.png
"""
import os, sys, json, base64, time, urllib.request, urllib.error

KEY = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
MODEL = "gemini-3-pro-image-preview"
OUT = os.path.join(os.path.dirname(__file__), "..", "public", "assets", "transformer-kamishibai")
os.makedirs(OUT, exist_ok=True)

STYLE = ("Japanese sumi-e ink-wash painting, traditional brush strokes, deep charcoal-black "
         "background, vast dark negative space, a single warm amber glow as the only light source, "
         "soft atmospheric mist, cinematic, minimal, painterly. A small lone figure in a simple robe, "
         "seen from behind, stands at the lower-center for scale. Absolutely NO text, NO letters, "
         "NO words, NO numbers, NO captions, NO symbols anywhere in the image. "
         "Wide 16:9 cinematic composition. The focal motif sits in the upper-center; "
         "keep the bottom third dark and simple for a subtitle overlay.")

SCENES = [
 "A web of small glowing word-orbs floating in the dark above the figure, connected by faint luminous threads into a soft swirling cloud, like a question made of light. The figure gazes up at it.",
 "A long single-file row of small glowing paper lanterns receding into the distance in one straight queue; the nearest lantern is bright, the farthest fade into blackness as if forgotten. The figure stands at the near end of the line.",
 "Many small glowing word-lanterns, no longer in a line, suddenly spread out and floating freely across a wide low plane all at once, the chain broken open — a quiet moment of release. The figure stands before the scattered glow.",
 "One bright glowing orb in the foreground casts many faint threads toward all the other dim orbs around it, but a SINGLE thread blazes brilliant amber, connecting it to one distant orb — selective focus. The figure watches.",
 "Three distinct glowing elements arranged as an elegant trio in the dark: a single round glowing orb, a row of small hanging tags, and a cluster of small boxes; a luminous thread pairs one tag to one box by brightness.",
 "A single horizontal line of glowing orbs viewed through several translucent overlapping glass panes set at different angles, each pane faintly tinted, multiple soft beams of gaze converging on the same line from many directions.",
 "A row of glowing orbs each resting on a small seat, and beneath them a single continuous flowing sine wave of light threads through all the seats, marking rhythm and position across the row.",
 "A slow single-file queue of dim lanterns left far behind in darkness on one side, while a sweeping streak of warm radiant light rushes forward past it toward the horizon — a sense of breakthrough and acceleration. The figure faces the bright light ahead.",
 "A small glowing scroll resting on the ground like a seed, from which a great luminous tree grows upward, its branches dissolving into countless points of warm light like distant minds; a campfire-warm glow pools at the base. The small figure stands at the roots, looking up. Hopeful finale.",
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
