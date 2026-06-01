#!/usr/bin/env python3
"""Generate the OGP card for /meet — moon aesthetic matching the page.
Output: public/assets/meet-og.png (1200x630). Run locally; commit the PNG."""
import os, glob, math
from PIL import Image, ImageDraw, ImageFont, ImageFilter

W, H = 1200, 630
OUT = os.environ.get("OUT", "public/assets/meet-og.png")
AMBER = (232, 160, 76)
BG = (8, 8, 10)


def find_font(patterns):
    roots = ["/System/Library/Fonts", "/System/Library/Fonts/Supplemental",
             "/Library/Fonts", "/usr/share/fonts", "/usr/local/share/fonts"]
    for r in roots:
        for p in patterns:
            m = glob.glob(os.path.join(r, "**", p), recursive=True)
            if m:
                return m[0]
    return None


BOLD = find_font(["ヒラギノ角ゴシック W7.ttc", "ヒラギノ角ゴシック W6.ttc",
                  "Hiragino Sans GB.ttc", "NotoSansCJK-Bold.ttc", "*Noto*CJK*Bold*"])
REG = find_font(["ヒラギノ角ゴシック W3.ttc", "Hiragino Sans GB.ttc",
                 "NotoSansCJK-Regular.ttc", "*Noto*CJK*Regular*"])
print("[meet-og] bold:", BOLD, "\n[meet-og] reg :", REG)


def font(path, size):
    try:
        return ImageFont.truetype(path, size)
    except Exception:
        return ImageFont.load_default()


img = Image.new("RGB", (W, H), BG)

# subtle vertical sheen
sheen = Image.new("L", (1, H), 0)
for y in range(H):
    sheen.putpixel((0, y), int(10 * (1 - y / H)))
img = Image.composite(Image.new("RGB", (W, H), (22, 22, 26)), img, sheen.resize((W, H)))

# ── moon glow (right side) ──
cx, cy, r = 930, 250, 118
glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow)
for i in range(60, 0, -1):
    rr = r + i * 5
    a = int(46 * (i / 60) ** 2)
    gd.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], fill=(*AMBER, a))
glow = glow.filter(ImageFilter.GaussianBlur(8))
img = Image.alpha_composite(img.convert("RGBA"), glow).convert("RGB")

# moon body — light-to-amber radial fake via concentric circles
md = ImageDraw.Draw(img)
for i in range(r, 0, -1):
    t = i / r
    # center highlight at upper-left
    col = (
        int(255 - (255 - AMBER[0]) * t),
        int(246 - (246 - AMBER[1]) * t),
        int(232 - (232 - AMBER[2]) * t),
    )
    ox, oy = int(r * 0.22 * (1 - t)), int(r * 0.22 * (1 - t))
    md.ellipse([cx - i - ox, cy - i - oy, cx + i - ox, cy + i - oy], fill=col)

d = ImageDraw.Draw(img)

# ── top-left wordmark ──
d.text((90, 84), "YUKIHAMADA.JP", font=font(BOLD, 26), fill=(150, 150, 158))

# ── title ──
d.text((88, 250), "日程を、選ぶ。", font=font(BOLD, 92), fill=(240, 240, 244))

# ── subtitle ──
d.text((92, 372), "濱田優貴とのミーティング", font=font(BOLD, 38), fill=AMBER)

# ── meta line ──
d.text((92, 440), "オンライン / 対面　・　約60分　・　候補から選ぶだけ",
       font=font(REG, 27), fill=(150, 150, 158))

# ── bottom accent bar ──
d.rectangle([0, H - 8, W, H], fill=AMBER)

os.makedirs(os.path.dirname(OUT), exist_ok=True)
img.save(OUT, "PNG")
print("[meet-og] wrote", OUT, img.size)
