#!/usr/bin/env python3
"""Generate triangle choke MV images using Gemini NanoBanana (anime style).
Accurate triangle choke mechanics based on BJJ technique research."""

import os
import base64
import json
import time
import requests

API_KEY = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
OUTPUT_DIR = "assets/images/generated/triangle"
# Use Gemini 2.5 Flash Image (NanoBanana)
MODEL = "gemini-2.5-flash-image"

# === Accurate Triangle Choke Mechanics ===
# The triangle choke (三角絞め / sankaku-jime) from closed guard:
#
# ATTACKER = on bottom (Hamada/student, blue belt, white gi)
# DEFENDER = on top, inside guard (Murata/sensei, black belt, white gi)
#
# Key positions:
# 1. Closed guard: Attacker's legs wrapped around defender's waist
# 2. Break posture: Pull defender's head down
# 3. Wrist control → isolate one arm in, one arm out
# 4. Hip explosion: Hips UP, swing right leg over defender's LEFT shoulder/neck
# 5. Triangle frame: Right leg over neck, left leg under right armpit
# 6. Lock: Right ankle hooks behind left knee (figure-four)
# 7. Angle cut: Body shifts 90° perpendicular to opponent
# 8. Finish: Pull head down, squeeze knees, elevate hips → carotid compression

# Character descriptions
SENSEI = "a Japanese man (mid-40s, short black hair, lean athletic build, white BJJ gi with black belt, calm wise expression) [MURATA-SENSEI]"
STUDENT = "a young Japanese man (30s, short hair, athletic build, white BJJ gi with blue belt, determined focused expression) [HAMADA]"

# Common anime style directive
ANIME = """Anime art style, clean cel-shading, vibrant colors, detailed line work similar to sports manga like Hajime no Ippo or All Rounder Meguru.
Dynamic composition, dramatic lighting with warm dojo amber tones.
16:9 cinematic aspect ratio. High detail on hands, feet, and gi fabric folds.
The BJJ technique positions must be anatomically accurate and clear enough to serve as instructional reference."""

SCENES = [
    # === INTRO ===
    {
        "name": "tri_intro_dojo",
        "prompt": f"""Anime illustration of a traditional Japanese BJJ dojo interior.
Two practitioners face each other: {SENSEI} standing with arms relaxed, and {STUDENT} bowing respectfully.
Wooden floor with tatami mats, sunlight streaming through shoji screens casting long golden shadows.
A calligraphy scroll on the wall reads '三角' (triangle). {ANIME}"""
    },
    {
        "name": "tri_intro_breathe",
        "prompt": f"""Anime close-up of {STUDENT} sitting cross-legged on the mat with eyes closed, taking a deep meditative breath before training.
His chest expands as he inhales. Soft particles of dust float in warm light beams.
Peaceful, focused atmosphere. {ANIME}"""
    },

    # === VERSE 1: Setting up from closed guard ===
    {
        "name": "tri_v1_closedguard",
        "prompt": f"""Anime illustration showing BJJ closed guard position from a 3/4 overhead angle:
{STUDENT} lies on his back on the mat. His LEGS are wrapped tightly around {SENSEI}'s waist, ankles crossed behind the lower back.
{SENSEI} is kneeling upright between the student's legs, hands on the student's chest trying to open the guard.
Both wear white gis. The student's blue belt and sensei's black belt are clearly visible.
Clear instructional view showing the classic closed guard position. {ANIME}"""
    },
    {
        "name": "tri_v1_sleevegrip",
        "prompt": f"""Anime close-up of hands gripping gi sleeves in BJJ:
{STUDENT} (on his back in guard) has BOTH hands wrapped around {SENSEI}'s gi sleeves near the wrists.
The grip is firm but controlled - fingers wrapped around the fabric, thumbs inside.
Focus on the detailed hand positions and gi fabric texture.
Background shows the dojo mat from a low angle. {ANIME}"""
    },
    {
        "name": "tri_v1_uppercut",
        "prompt": f"""Anime side-view illustration of triangle choke setup:
{STUDENT} is on his back in closed guard. His RIGHT hand pushes upward against {SENSEI}'s chest/solar plexus area at an UPPERCUT ANGLE - fist against the sternum, arm straight, pushing sensei's posture backward.
His LEFT hand still grips sensei's right sleeve.
The push creates space between their bodies - this gap is crucial for the triangle entry.
Show motion lines on the pushing arm. {ANIME}"""
    },
    {
        "name": "tri_v1_push_pull",
        "prompt": f"""Anime dynamic illustration of the push-pull triangle setup:
{STUDENT} on his back: RIGHT hand pushes {SENSEI}'s chest away (straight arm, palm on sternum).
LEFT hand PULLS sensei's right arm toward him like drawing a bow (pulling the wrist across).
This creates the critical "one arm in, one arm out" position.
Sensei's LEFT arm is pushed to his own body. Sensei's RIGHT arm is pulled forward across student's center.
Show red arrows indicating the push and pull directions. {ANIME}"""
    },

    # === HOOK: Triangle entry and lock ===
    {
        "name": "tri_hook_hip_up",
        "prompt": f"""Anime dynamic action shot of triangle choke entry:
{STUDENT} on his back EXPLOSIVELY lifts his hips HIGH off the mat (hip bump).
His closed guard opens. His RIGHT LEG swings UP and OVER {SENSEI}'s left shoulder, the calf draping over the back of sensei's neck.
His LEFT LEG drops down, knee pointing outward.
Sensei's right arm is pulled forward (trapped). Sensei's left arm is pushed against his own body.
Speed lines and impact effects around the hip explosion. This is the critical entry moment. {ANIME}"""
    },
    {
        "name": "tri_hook_leg_over_neck",
        "prompt": f"""Anime illustration showing the triangle choke position from the side:
{STUDENT} is on his back. His RIGHT LEG is draped over the BACK OF {SENSEI}'s NECK, with the hamstring pressing against the left side of sensei's neck.
Sensei's RIGHT ARM is trapped INSIDE the triangle (between student's right thigh and sensei's own neck).
Sensei's LEFT ARM is OUTSIDE the triangle.
The student's left leg is underneath, ready to lock.
This is the classic "one arm in, one arm out" triangle frame. {ANIME}"""
    },
    {
        "name": "tri_hook_figure4_lock",
        "prompt": f"""Anime detailed close-up of the triangle choke LOCK mechanism:
{STUDENT}'s RIGHT ANKLE hooks behind his own LEFT KNEE, creating a figure-four leg lock.
Right leg: over sensei's neck, ankle hooked behind left knee.
Left leg: shin pressed against sensei's right shoulder/arm.
Together the legs form a TRIANGLE SHAPE around sensei's neck and one trapped arm.
Show this from a slightly low angle with clear view of the ankle-behind-knee connection.
Highlight the triangle shape with a subtle glowing triangle overlay. {ANIME}"""
    },
    {
        "name": "tri_hook_angle_cut",
        "prompt": f"""Anime overhead/bird's eye view of the triangle choke with angle cut:
{STUDENT} has shifted his body to a 90-DEGREE ANGLE perpendicular to {SENSEI}.
His legs form a tight triangle around sensei's neck (right leg over neck, locked behind left knee).
Student's body points to the LEFT (not straight under sensei but angled off to the side).
Student pushes off sensei's hip with his left hand to create this angle.
Show the 90° angle clearly with a dotted line guide. This angle is what makes the choke tight. {ANIME}"""
    },

    # === HOOK continued: The squeeze and tap ===
    {
        "name": "tri_hook_head_pull",
        "prompt": f"""Anime illustration of triangle choke finishing details:
{STUDENT} uses BOTH HANDS to pull {SENSEI}'s head DOWN into the triangle.
Hands clasped behind sensei's head, pulling it toward student's own chest.
Student's legs squeeze tight - knees pinching together.
Student's hips are elevated off the mat, increasing the choke pressure.
Sensei's face shows the pressure building - brow furrowed, controlled discomfort.
Dramatic lighting with shadows emphasizing the intensity. {ANIME}"""
    },
    {
        "name": "tri_hook_squeeze",
        "prompt": f"""Anime dramatic illustration of the triangle choke at maximum pressure:
{STUDENT}'s inner thighs squeeze {SENSEI}'s neck from both sides.
The right hamstring presses against the left carotid artery.
The left shin presses against sensei's trapped right arm, which presses against the right carotid.
This creates bilateral compression of both carotid arteries (blood choke, not air choke).
Student's face is calm and focused. Sensei acknowledges the perfect technique.
Show subtle red pressure indicators on both sides of the neck. {ANIME}"""
    },
    {
        "name": "tri_hook_sensei_wisdom",
        "prompt": f"""Anime split-panel composition:
LEFT PANEL (memory/flashback, soft watercolor style): {SENSEI} teaching in the dojo, pointing at a diagram of the triangle choke on a whiteboard, explaining with a gentle smile.
RIGHT PANEL (present, sharp anime style): {STUDENT} executing the exact technique sensei taught, legs locked in perfect triangle around a training partner.
A translucent connection line between the two panels showing the teacher-student bond.
'The wisdom of the teacher lives in the student's technique.' {ANIME}"""
    },
    {
        "name": "tri_hook_tap",
        "prompt": f"""Anime emotional close-up of THE TAP moment:
{SENSEI}'s right hand reaches out and TAPS three times on {STUDENT}'s thigh.
The hand is open, palm flat, tapping clearly - the universal signal of submission.
Show three impact marks (tap, tap, tap) with small ripple effects.
Despite being caught in the triangle, sensei has a slight proud smile - his student has mastered the technique.
Warm golden light. This is a moment of respect, not defeat. {ANIME}"""
    },

    # === VERSE 2: Open guard triangle variation ===
    {
        "name": "tri_v2_foot_on_hip",
        "prompt": f"""Anime illustration of open guard triangle setup:
{STUDENT} lies on his back with his RIGHT FOOT pressed against {SENSEI}'s LEFT HIP.
His left foot is on the mat, knee bent. This is open guard position (not closed guard).
His hands grip both of sensei's sleeves at the wrists.
He's creating distance and angle with the foot on hip, preparing for a different triangle entry.
Show the foot clearly planted on the hip bone area. {ANIME}"""
    },
    {
        "name": "tri_v2_leg_swing",
        "prompt": f"""Anime dynamic action shot with heavy motion blur:
{STUDENT} swings his RIGHT LEG in a wide arc UP and OVER {SENSEI}'s shoulder toward the neck.
The leg traces a TRIANGULAR PATH through the air (show the motion trail as a glowing triangle shape).
This is a faster, more explosive triangle entry from open guard.
Student pulls sensei's right arm with his left hand simultaneously.
Maximum dynamism - speed lines, wind effects, fabric flutter. {ANIME}"""
    },
    {
        "name": "tri_v2_head_control",
        "prompt": f"""Anime illustration of head control in the triangle:
{STUDENT} has the triangle locked. Both his hands are behind {SENSEI}'s head, fingers interlaced.
He pulls sensei's head DOWN firmly but gently toward his own chest.
Sensei's chin is tucked against his own chest, face looking down.
The pulling motion tightens the triangle significantly.
Show from a side angle with emphasis on the hand position behind the head. {ANIME}"""
    },
    {
        "name": "tri_v2_no_escape",
        "prompt": f"""Anime illustration showing the completed triangle from above:
Bird's eye view looking down at the triangle choke fully locked.
{STUDENT}'s legs clearly form a TRIANGLE SHAPE around {SENSEI}'s neck.
Inside the triangle: sensei's head + one trapped arm.
Outside the triangle: sensei's other arm reaching but finding no escape.
Student's body is angled 90° to the side.
The composition emphasizes the geometric beauty of the triangle shape.
Subtle glowing triangle overlay. No escape routes visible. {ANIME}"""
    },
    {
        "name": "tri_v2_calm_control",
        "prompt": f"""Anime portrait of {STUDENT} during the final moments of the triangle choke:
Close-up on his face from below. Eyes calm and focused, breathing steady.
His legs (out of focus in foreground) hold the triangle lock.
Behind him, the dojo ceiling and lights create a halo effect.
'Calm control is the path to victory' - this is a moment of mastery, not violence.
Serene expression, slight sweat on brow, total confidence. {ANIME}"""
    },

    # === OUTRO: Philosophy ===
    {
        "name": "tri_outro_mypath",
        "prompt": f"""Anime hero shot of {STUDENT} standing alone on the mat after training:
Full body, slightly from below (heroic angle). White gi slightly open, blue belt tied.
Sweat glistening, one hand wiping his brow. Determined expression looking ahead.
Behind him, a long corridor of the dojo stretches into golden light.
'This is my path. This is my technique.'
Dramatic backlighting creating a silhouette effect with rim light. {ANIME}"""
    },
    {
        "name": "tri_outro_meditation",
        "prompt": f"""Anime peaceful scene: {STUDENT} sitting in seiza (kneeling) on the mat.
Eyes closed, hands on thighs, perfectly still. Meditation pose.
Around him, faint geometric triangle shapes float like cherry blossom petals.
The dojo is empty and quiet. Last rays of sunset paint everything amber and purple.
'Preparation is complete. The mind is clear like still water.'
Zen atmosphere, minimal composition. {ANIME}"""
    },
    {
        "name": "tri_outro_triangle_proof",
        "prompt": f"""Anime artistic overhead illustration:
{STUDENT} executing a perfect triangle choke on {SENSEI}.
Shot directly from above, the legs form a PERFECT GEOMETRIC TRIANGLE.
The triangle shape is emphasized with glowing blue/gold energy lines.
Around them, the mat shows radiating energy circles.
'The triangle is gentle proof — when it's locked, they tap.'
Abstract and beautiful, emphasizing the art and geometry of the technique. {ANIME}"""
    },
    {
        "name": "tri_outro_bow",
        "prompt": f"""Anime emotional final scene:
{SENSEI} and {STUDENT} sitting face to face on the mat in seiza, bowing to each other.
Deep respectful bow. Sunset light floods through dojo windows.
Between them on the mat, a subtle triangle symbol glows faintly.
'An unwavering tap. OSS.'
Warm colors, lens flare, emotional climax. The bond between teacher and student. {ANIME}"""
    },

    # === TITLE CARD ===
    {
        "name": "tri_title",
        "prompt": f"""Anime title card illustration:
Dark background with a dramatic silhouette of two BJJ practitioners - one executing a triangle choke on the other.
The legs clearly form a triangle shape, glowing with blue energy.
Large elegant Japanese text: '三角のうた' (Triangle Song)
Smaller text below: 'Triangle Song'
Anime movie poster style with dramatic lighting.
Color scheme: deep navy background, glowing blue triangle, gold accents. {ANIME}"""
    },
]


def generate_image_nanobanana(scene):
    """Generate anime image using Gemini NanoBanana (2.5 Flash Image)."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={API_KEY}"

    payload = {
        "contents": [{
            "parts": [{"text": scene["prompt"]}]
        }],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"],
        }
    }

    headers = {"Content-Type": "application/json"}

    try:
        resp = requests.post(url, json=payload, headers=headers, timeout=180)
        resp.raise_for_status()
        data = resp.json()

        for candidate in data.get("candidates", []):
            for part in candidate.get("content", {}).get("parts", []):
                if "inlineData" in part:
                    img_data = base64.b64decode(part["inlineData"]["data"])
                    filepath = os.path.join(OUTPUT_DIR, f"{scene['name']}.jpg")
                    with open(filepath, "wb") as f:
                        f.write(img_data)
                    size_kb = len(img_data) // 1024
                    print(f"  OK: {filepath} ({size_kb}KB)")
                    return True

        # Check for safety blocks
        block = data.get("candidates", [{}])[0].get("finishReason", "")
        if block == "SAFETY":
            print(f"  BLOCKED by safety filter")
            return False

        print(f"  No image in response")
        text_parts = []
        for c in data.get("candidates", []):
            for p in c.get("content", {}).get("parts", []):
                if "text" in p:
                    text_parts.append(p["text"][:200])
        if text_parts:
            print(f"  Text response: {text_parts[0]}")
        return False

    except Exception as e:
        print(f"  ERROR: {e}")
        return False


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    print(f"=== NanoBanana Anime Triangle Choke MV ===")
    print(f"Model: {MODEL}")
    print(f"Scenes: {len(SCENES)}")
    print(f"Output: {OUTPUT_DIR}")
    print()

    success = 0
    failed = []

    for i, scene in enumerate(SCENES):
        filepath = os.path.join(OUTPUT_DIR, f"{scene['name']}.jpg")

        print(f"[{i+1}/{len(SCENES)}] {scene['name']}...")
        if generate_image_nanobanana(scene):
            success += 1
        else:
            failed.append(scene['name'])
            print(f"  FAILED")

        # Rate limiting (Gemini has per-minute quotas)
        time.sleep(3)

    print(f"\n=== Results ===")
    print(f"Success: {success}/{len(SCENES)}")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
