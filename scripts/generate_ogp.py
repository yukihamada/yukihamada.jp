#!/usr/bin/env python3
"""Auto-generate slug-based OGP images for blog posts that don't have one yet."""
import os, re, glob
from PIL import Image, ImageDraw, ImageFont

BLOG_DIR = os.environ.get("BLOG_DIR", "content/blog")
IMG_DIR  = os.environ.get("IMG_DIR",  "public/blog/images")
W, H = 1200, 630

def find_font(name_patterns):
    """Search common font directories for a matching font file."""
    search_roots = [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/System/Library/Fonts",
        "/Library/Fonts",
    ]
    for root in search_roots:
        for pattern in name_patterns:
            matches = glob.glob(os.path.join(root, "**", pattern), recursive=True)
            if matches:
                return matches[0]
    return None

FONT_BOLD_PATH = find_font(["NotoSansCJK-Bold.ttc", "NotoSansCJKjp-Bold.otf",
                             "NotoSansCJK-Bold.otf", "*Noto*CJK*Bold*",
                             "ヒラギノ角ゴシック W7.ttc"])
FONT_REG_PATH  = find_font(["NotoSansCJK-Regular.ttc", "NotoSansCJKjp-Regular.otf",
                             "NotoSansCJK-Regular.otf", "*Noto*CJK*Regular*",
                             "ヒラギノ角ゴシック W3.ttc"])

if FONT_BOLD_PATH:
    print(f"[ogp] Using bold font: {FONT_BOLD_PATH}")
else:
    print("[ogp] No CJK bold font found, using Pillow default")
if FONT_REG_PATH:
    print(f"[ogp] Using regular font: {FONT_REG_PATH}")
else:
    print("[ogp] No CJK regular font found, using Pillow default")

def load_font(path, size):
    if path:
        try:
            return ImageFont.truetype(path, size)
        except Exception as e:
            print(f"[ogp] Warning: could not load {path}: {e}")
    return ImageFont.load_default()

f_title = load_font(FONT_BOLD_PATH, 58)
f_sub   = load_font(FONT_BOLD_PATH, 28)
f_small = load_font(FONT_REG_PATH,  22)
f_logo  = load_font(FONT_BOLD_PATH, 22)

def parse_fm(path):
    with open(path, encoding="utf-8") as f:
        content = f.read()
    m = re.match(r'^---\s*\n(.*?)\n---', content, re.DOTALL)
    if not m:
        return {}
    fm = {}
    for line in m.group(1).splitlines():
        line = line.strip()
        for key in ("title", "date", "description"):
            if line.startswith(key + ":"):
                fm[key] = line[len(key)+1:].strip().strip('"').strip("'")
        if line.startswith("tags:"):
            raw = line[5:].strip().strip("[]")
            fm["tags"] = [t.strip().strip("\"'") for t in raw.split(",") if t.strip()]
    return fm

def wrap(draw, text, font, max_w):
    lines, cur = [], ""
    for ch in text:
        test = cur + ch
        w = draw.textbbox((0,0), test, font=font)[2]
        if w > max_w and cur:
            lines.append(cur); cur = ch
        else:
            cur = test
    if cur:
        lines.append(cur)
    return lines

def make_ogp(slug, date, title, desc, tags):
    img = Image.new("RGB", (W, H), (13, 13, 13))
    d = ImageDraw.Draw(img)
    px = 72

    title_lines = wrap(d, title, f_title, W - px*2)
    y = 120
    for line in title_lines[:2]:
        d.text((px, y), line, font=f_title, fill=(255, 255, 255))
        y += 75

    desc_short = desc[:80] + ("…" if len(desc) > 80 else "")
    desc_lines = wrap(d, desc_short, f_small, W - px*2)
    y = max(y + 24, 300)
    for line in desc_lines[:2]:
        d.text((px, y), line, font=f_small, fill=(160, 160, 160))
        y += 32

    tags_str = "  ·  ".join(tags[:5]) if tags else ""
    d.text((px, max(y + 16, 390)), tags_str, font=f_small, fill=(90, 90, 90))

    bar_y = H - 70
    d.rounded_rectangle([px, bar_y, px+42, bar_y+42], radius=6, fill=(255, 255, 255))
    d.text((px+8, bar_y+9), "YH", font=f_logo, fill=(13, 13, 13))
    d.text((px+56, bar_y+10), "yukihamada.jp", font=f_small, fill=(200, 200, 200))
    tw = d.textbbox((0,0), date, font=f_small)[2]
    d.text((W - px - tw, bar_y+10), date, font=f_small, fill=(120, 120, 120))

    os.makedirs(IMG_DIR, exist_ok=True)
    out = os.path.join(IMG_DIR, f"ogp-{slug}.png")
    img.save(out, "PNG", optimize=True)
    return out

created = 0
for fname in sorted(os.listdir(BLOG_DIR)):
    if not fname.endswith(".md"):
        continue
    slug = fname[:-3]
    dst  = os.path.join(IMG_DIR, f"ogp-{slug}.png")
    if os.path.exists(dst):
        continue
    fm = parse_fm(os.path.join(BLOG_DIR, fname))
    if not fm.get("date"):
        continue
    out = make_ogp(slug, fm["date"], fm.get("title",""), fm.get("description",""), fm.get("tags",[]))
    print(f"[ogp] generated {os.path.basename(out)}")
    created += 1

print(f"[ogp] done — {created} new images generated")
