#!/usr/bin/env python3
"""Generate triangle choke MV images using Gemini 2.5 Flash Image Generation."""

import os
import base64
import json
import time
import requests

API_KEY = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
OUTPUT_DIR = "assets/images/generated/triangle"

# Character descriptions for consistency
MURATA_SENSEI = "a Japanese BJJ black belt instructor (mid-40s, 177cm, lean athletic build, short black hair, wearing a white gi with a black belt, calm confident expression)"
HAMADA_YUKI = "a Japanese BJJ student (30s, short hair, athletic build, wearing a white gi with a blue belt, focused expression)"

# Common style directive
STYLE = "Photorealistic illustration, cinematic lighting, warm dojo atmosphere with wooden floors, dramatic shadows, Japanese martial arts aesthetic. 16:9 aspect ratio, high quality."

# Scene definitions matching the song structure
SCENES = [
    # Intro (0-15s) - Dojo atmosphere, breathing
    {
        "name": "tri_intro_dojo",
        "prompt": f"Wide shot of a traditional Japanese BJJ dojo. {MURATA_SENSEI} stands at the center facing {HAMADA_YUKI}. Both bow respectfully. Warm golden light filtering through windows. {STYLE}"
    },
    {
        "name": "tri_intro_breathe",
        "prompt": f"Close-up of {HAMADA_YUKI} sitting in closed guard position, eyes closed, taking a deep breath. {MURATA_SENSEI} watching from above. Peaceful meditative moment before training. {STYLE}"
    },

    # Verse 1 - Setting up triangle from closed guard
    {
        "name": "tri_v1_closedguard",
        "prompt": f"Top-down view of BJJ training: {HAMADA_YUKI} is on his back in closed guard position with legs wrapped around {MURATA_SENSEI} who is in the guard. Both wearing white gis. Clear view of the closed guard leg position. Instructional clarity. {STYLE}"
    },
    {
        "name": "tri_v1_sleevegrip",
        "prompt": f"Close-up of hands gripping gi sleeves in BJJ. {HAMADA_YUKI} (on bottom in guard) gently wraps both hands around {MURATA_SENSEI}'s sleeves, controlling the arms. Detailed view of the grip technique. Soft lighting. {STYLE}"
    },
    {
        "name": "tri_v1_uppercut_angle",
        "prompt": f"BJJ technique detail: {HAMADA_YUKI} on his back, one hand positioned at uppercut angle pushing against {MURATA_SENSEI}'s chest/solar plexus area. Clear demonstration of hand placement for triangle setup. Side angle view. {STYLE}"
    },
    {
        "name": "tri_v1_bow_pull",
        "prompt": f"BJJ technique: {HAMADA_YUKI} on his back, one hand pulling {MURATA_SENSEI}'s arm like drawing a bow (hikitsukeru motion). The other hand on the chest. Clear demonstration of the pull-and-push setup for triangle choke. {STYLE}"
    },

    # Hook - Triangle lock sequence
    {
        "name": "tri_hook_hip_up",
        "prompt": f"Dynamic BJJ moment: {HAMADA_YUKI} lifting his hips high (hip bump/elevator motion), one leg shooting up over {MURATA_SENSEI}'s shoulder toward the neck. The beginning of triangle choke entry. Dramatic angle from the side. {STYLE}"
    },
    {
        "name": "tri_hook_leg_catch",
        "prompt": f"BJJ triangle choke in progress: {HAMADA_YUKI}'s legs forming a triangle shape around {MURATA_SENSEI}'s neck and one arm. One leg over the shoulder, the other hooking behind the knee. Clear triangle lock position. Cinematic side view. {STYLE}"
    },
    {
        "name": "tri_hook_knee_fold",
        "prompt": f"Close-up of the triangle choke lock: {HAMADA_YUKI} folding his knee tight, pulling {MURATA_SENSEI}'s head down close. The triangle is getting tighter. Detailed view of leg position and head control. {STYLE}"
    },
    {
        "name": "tri_hook_door_close",
        "prompt": f"The completed triangle choke: {HAMADA_YUKI} has a tight triangle locked around {MURATA_SENSEI}'s neck. Legs forming a perfect triangle shape. Serene expression on the bottom player. Like quietly closing a door. Beautiful composition. {STYLE}"
    },

    # Hook continued - Outside to inside leg, clutch
    {
        "name": "tri_hook_gaisen_naisen",
        "prompt": f"BJJ detail: transitioning leg position from outside (gaisen) to inside (naisen) during triangle choke. {HAMADA_YUKI}'s leg smoothly rotating inward. Technical close-up showing the leg angle change. Arrows or visual flow showing direction. {STYLE}"
    },
    {
        "name": "tri_hook_sensei_trust",
        "prompt": f"Emotional moment: split composition - on one side {MURATA_SENSEI} teaching with a pointing gesture (flashback/memory), on the other side {HAMADA_YUKI} executing the triangle choke with confidence. Trust in the teacher's wisdom. Warm golden tones. {STYLE}"
    },
    {
        "name": "tri_hook_inner_thigh",
        "prompt": f"BJJ triangle choke squeeze: {HAMADA_YUKI}'s inner thighs squeezing tight, hips supporting the lock. Close-up showing the pressure mechanics. {MURATA_SENSEI} beginning to tap. Dramatic lighting. {STYLE}"
    },
    {
        "name": "tri_hook_tap",
        "prompt": f"The tap moment: {MURATA_SENSEI} tapping on {HAMADA_YUKI}'s leg three times (tap, tap, tap) while caught in the triangle choke. Close-up of the hand tapping. Respectful submission. Emotional moment. {STYLE}"
    },

    # Verse 2 - Standing/open triangle variation
    {
        "name": "tri_v2_foot_hip",
        "prompt": f"BJJ triangle setup variation: {HAMADA_YUKI} placing his foot on {MURATA_SENSEI}'s hip, opening the angle softly. Open guard to triangle transition. Wide shot showing full body positions. {STYLE}"
    },
    {
        "name": "tri_v2_leg_swing",
        "prompt": f"Dynamic action shot: {HAMADA_YUKI} swinging his leg up high, the leg tracing a triangular arc in the air. Motion blur on the leg. {MURATA_SENSEI} caught in the movement. Dramatic angle. {STYLE}"
    },
    {
        "name": "tri_v2_head_pull",
        "prompt": f"BJJ triangle choke control: {HAMADA_YUKI} pulling {MURATA_SENSEI}'s head down gently but firmly into the triangle. Both hands behind the head. Close-up showing the control and gradual pressure. {STYLE}"
    },
    {
        "name": "tri_v2_pressure",
        "prompt": f"Tight triangle choke: the pressure building, {MURATA_SENSEI}'s face showing the squeeze effect. {HAMADA_YUKI} maintaining calm control. No escape routes visible. Tension in the scene. Cinematic lighting. {STYLE}"
    },
    {
        "name": "tri_v2_control",
        "prompt": f"The final moment of a perfectly controlled triangle choke. {HAMADA_YUKI} maintaining composure. {MURATA_SENSEI} acknowledging the technique with a slight nod. Road to victory through calm control. Wide shot of both practitioners. {STYLE}"
    },

    # Outro - Philosophy and path
    {
        "name": "tri_outro_mypath",
        "prompt": f"{HAMADA_YUKI} standing alone on the mat after training, white gi slightly disheveled, looking determined. Sweat glistening. 'This is my path, this is my technique.' Solo hero shot. Dramatic backlighting. {STYLE}"
    },
    {
        "name": "tri_outro_ready",
        "prompt": f"{HAMADA_YUKI} in meditation pose (seiza) on the mat, eyes closed, perfectly still. 'Preparation is complete, the mind is clear.' Zen-like atmosphere. Minimal composition. {STYLE}"
    },
    {
        "name": "tri_outro_proof",
        "prompt": f"Artistic overhead shot: {HAMADA_YUKI} executing a perfect triangle choke on {MURATA_SENSEI}. The legs form a beautiful geometric triangle. 'The triangle is gentle proof.' Artistic composition emphasizing the triangle shape. {STYLE}"
    },
    {
        "name": "tri_outro_final_tap",
        "prompt": f"Final scene: both {MURATA_SENSEI} and {HAMADA_YUKI} sitting together on the mat, bowing to each other. Respect between teacher and student. Warm sunset light through dojo windows. 'An unwavering tap.' Peaceful conclusion. {STYLE}"
    },

    # Title/overlay images
    {
        "name": "tri_title",
        "prompt": f"Title card design: 'Triangle Song - 三角のうた' in elegant Japanese typography. Background shows a geometric triangle shape formed by BJJ legs (abstract/silhouette). Dark background with golden accent light. Cinematic. {STYLE}"
    },
]

def generate_image(scene):
    """Generate image using Imagen 4.0."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict?key={API_KEY}"

    payload = {
        "instances": [{
            "prompt": scene["prompt"]
        }],
        "parameters": {
            "sampleCount": 1,
            "aspectRatio": "16:9",
            "personGeneration": "allow_all"
        }
    }

    headers = {"Content-Type": "application/json"}

    try:
        resp = requests.post(url, json=payload, headers=headers, timeout=120)
        resp.raise_for_status()
        data = resp.json()

        # Extract image from Imagen response
        predictions = data.get("predictions", [])
        if predictions:
            img_data = base64.b64decode(predictions[0]["bytesBase64Encoded"])
            filepath = os.path.join(OUTPUT_DIR, f"{scene['name']}.jpg")
            with open(filepath, "wb") as f:
                f.write(img_data)
            print(f"  Saved: {filepath} ({len(img_data)} bytes)")
            return True

        # Fallback: try Gemini 2.5 Flash Image
        print(f"  No image from Imagen, trying Gemini 2.5 Flash Image...")
        return generate_image_gemini(scene)

    except Exception as e:
        print(f"  Imagen error for {scene['name']}: {e}")
        # Fallback to Gemini
        return generate_image_gemini(scene)


def generate_image_gemini(scene):
    """Fallback: Generate image using Gemini 2.5 Flash Image."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent?key={API_KEY}"

    payload = {
        "contents": [{
            "parts": [{
                "text": f"Generate an image: {scene['prompt']}"
            }]
        }],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"],
        }
    }

    headers = {"Content-Type": "application/json"}

    try:
        resp = requests.post(url, json=payload, headers=headers, timeout=120)
        resp.raise_for_status()
        data = resp.json()

        for candidate in data.get("candidates", []):
            for part in candidate.get("content", {}).get("parts", []):
                if "inlineData" in part:
                    img_data = base64.b64decode(part["inlineData"]["data"])
                    filepath = os.path.join(OUTPUT_DIR, f"{scene['name']}.jpg")
                    with open(filepath, "wb") as f:
                        f.write(img_data)
                    print(f"  Saved (Gemini): {filepath} ({len(img_data)} bytes)")
                    return True

        print(f"  No image from Gemini either for {scene['name']}")
        print(f"  Response: {json.dumps(data, indent=2)[:300]}")
        return False

    except Exception as e:
        print(f"  Gemini error for {scene['name']}: {e}")
        return False


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    print(f"Generating {len(SCENES)} triangle choke MV images...")
    print(f"Output: {OUTPUT_DIR}")
    print()

    success = 0
    for i, scene in enumerate(SCENES):
        # Skip if already exists
        filepath = os.path.join(OUTPUT_DIR, f"{scene['name']}.jpg")
        if os.path.exists(filepath) and os.path.getsize(filepath) > 1000:
            print(f"[{i+1}/{len(SCENES)}] {scene['name']} - already exists, skipping")
            success += 1
            continue

        print(f"[{i+1}/{len(SCENES)}] Generating: {scene['name']}...")
        if generate_image(scene):
            success += 1
        else:
            print(f"  FAILED - will retry later")

        # Rate limiting
        time.sleep(2)

    print(f"\nDone! {success}/{len(SCENES)} images generated.")


if __name__ == "__main__":
    main()
