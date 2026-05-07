#!/usr/bin/env python3
"""Generate background images for soluna.html using Gemini image API."""
import os, time, sys

try:
    from google import genai
    from google.genai import types
except ImportError:
    os.system("pip install google-genai -q")
    from google import genai
    from google.genai import types

API_KEY = os.environ.get('GEMINI_API_KEY') or os.environ.get('GOOGLE_API_KEY')
if not API_KEY:
    print("ERROR: GEMINI_API_KEY not found"); sys.exit(1)

client = genai.Client(api_key=API_KEY)
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'assets/images/generated/')
os.makedirs(OUT, exist_ok=True)

SCENES = [
    ('soluna_s0_moon',
     'Cinematic night sky over Tokyo. Massive full moon with silver-blue glow casts moonlight on glass skyscrapers. Deep indigo and midnight blue tones. Photorealistic, 16:9 widescreen.'),
    ('soluna_s1_property',
     'Luxury modern apartment building in Tokyo at night, warm golden lights glowing from windows, sleek contemporary architecture, soft bokeh street lights below. Photorealistic, 16:9.'),
    ('soluna_s2_pon',
     'Digital contract signing on glass tablet, glowing blue electronic signature, dark minimal tech aesthetic, hands holding stylus over glowing document screen, futuristic UI. Photorealistic, 16:9.'),
    ('soluna_s3_kagi',
     'Smart home interior, warm amber lighting, futuristic door smart lock glowing teal, iPhone wall panel showing home controls, modern Japanese minimalist apartment at night. Photorealistic, 16:9.'),
    ('soluna_s4_beach',
     'Tropical beach at golden hour, crystal turquoise water, MacBook open on wooden beach table, palm trees framing shot, Bali or Hawaii vibes, cinematic bokeh, digital nomad paradise. Photorealistic, 16:9.'),
    ('soluna_s5_cafe',
     'Cozy modern cafe interior, laptop on wooden table with latte art coffee cup, warm Edison bulb lighting, digital nomad working, shallow depth of field, productive atmosphere. Photorealistic, 16:9.'),
    ('soluna_s6_dashboard',
     'Financial income dashboard on MacBook screen, glowing green upward charts and graphs, passive income real estate analytics, dark UI with mint green data visualization, home office. Photorealistic, 16:9.'),
    ('soluna_s7_surf',
     'Surfer riding a perfect turquoise wave at sunset in Bali Indonesia, dramatic orange and gold sky, cinematic wide shot from water level, white foam spray, freedom and adventure. Photorealistic, 16:9.'),
    ('soluna_s8_buildings',
     'Aerial view of luxury apartment buildings in Tokyo at twilight, city grid from above, warm lights in windows, real estate portfolio concept, golden hour drone photography. Photorealistic, 16:9.'),
    ('soluna_s9_salary',
     'Japanese professional businessman at desk in Tokyo high-rise office, confident smile, nighttime city skyline through floor-to-ceiling window, dark suit, success and ambition. Photorealistic, 16:9.'),
    ('soluna_s10_startup',
     'Dynamic startup entrepreneur in modern loft office, multiple monitors showing business dashboards, whiteboard with growth charts, energetic atmosphere, purple and amber accent lighting. Photorealistic, 16:9.'),
    ('soluna_s11_sunset',
     'Silhouette of a man on a cliff overlooking the Pacific Ocean at dramatic Hawaiian sunset, arms wide open, ultra cinematic wide shot, deep orange gold purple sky, lens flare, freedom. Photorealistic, 16:9.'),
    ('soluna_s12_outro',
     'Full moon reflection on calm ocean at night, three glowing holographic icons floating in the night sky representing buildings, signature pen, and smart key, city lights on horizon, dreamy atmospheric. Photorealistic, 16:9.'),
]

generated, failed = [], []

for name, prompt in SCENES:
    path = f'{OUT}{name}.jpg'
    if os.path.exists(path) and os.path.getsize(path) > 10000:
        print(f'  SKIP (exists {os.path.getsize(path)//1024}KB): {name}')
        generated.append(name)
        continue

    print(f'  Generating: {name}...', flush=True)
    for attempt in range(3):
        try:
            resp = client.models.generate_content(
                model='gemini-3-pro-image-preview',
                contents=prompt,
                config=types.GenerateContentConfig(
                    response_modalities=['IMAGE', 'TEXT']
                )
            )
            saved = False
            if resp.candidates and resp.candidates[0].content:
                for part in resp.candidates[0].content.parts:
                    if part.inline_data and part.inline_data.data:
                        # data is already raw bytes, NOT base64
                        with open(path, 'wb') as f:
                            f.write(part.inline_data.data)
                        size = os.path.getsize(path)
                        print(f'    OK: {size//1024}KB saved to {name}.jpg')
                        generated.append(name)
                        saved = True
                        break
            if not saved:
                reason = resp.candidates[0].finish_reason if resp.candidates else 'no candidates'
                print(f'    No image data: finish_reason={reason}')
                failed.append(name)
            break
        except Exception as e:
            print(f'    Attempt {attempt+1} error: {e}')
            if attempt < 2:
                time.sleep(8)
            else:
                failed.append(name)
    time.sleep(4)

print(f'\nDone. Generated: {len(generated)}/{len(SCENES)}, Failed: {len(failed)}')
if failed:
    print(f'Failed: {failed}')
