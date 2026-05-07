#!/usr/bin/env python3
"""
Blog → Podcast audio generator
Usage: python3 scripts/gen_podcast.py <blog-slug>
       python3 scripts/gen_podcast.py 2026-05-01-bjj-best-sport-science

Reads content/blog/<slug>.md, converts to podcast script via Claude,
generates audio via ElevenLabs in ~200-char chunks, concatenates with ffmpeg.
Output: public/audio/blog/<slug>.mp3
"""

import os
import sys
import re
import json
import time
import tempfile
import subprocess
import anthropic
import requests

# ── Config ──────────────────────────────────────────────
ELEVENLABS_API_KEY = os.environ.get("ELEVENLABS_API_KEY", "")
ANTHROPIC_API_KEY  = os.environ.get("ANTHROPIC_API_KEY", "")

VOICE_ID   = "VneiyrGsB8R1ym9S1XYl"   # Yuki Hamada – cloned voice
MODEL_ID   = "eleven_multilingual_v2"
CHUNK_SIZE = 200                         # target chars per TTS request

VOICE_SETTINGS = {
    "stability": 0.55,
    "similarity_boost": 0.80,
    "style": 0.20,
    "use_speaker_boost": True,
}

# ── Helpers ──────────────────────────────────────────────

def load_env():
    env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
    if not os.path.exists(env_path):
        env_path = os.path.expanduser("~/.env")
    if os.path.exists(env_path):
        with open(env_path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    k, _, v = line.partition("=")
                    os.environ.setdefault(k.strip(), v.strip())

def read_markdown(slug):
    base = os.path.join(os.path.dirname(__file__), "..", "content", "blog")
    path = os.path.join(base, f"{slug}.md")
    if not os.path.exists(path):
        sys.exit(f"Not found: {path}")
    with open(path, encoding="utf-8") as f:
        return f.read()

def strip_frontmatter(text):
    if text.startswith("---"):
        end = text.index("---", 3)
        return text[end + 3:].strip()
    return text

def extract_title(text):
    m = re.search(r'^title:\s*["\']?(.+?)["\']?\s*$', text, re.MULTILINE)
    return m.group(1) if m else "ブログ記事"

def md_to_text(md):
    """Strip markdown syntax to plain text."""
    # Remove frontmatter
    if md.startswith("---"):
        md = md[md.index("---", 3) + 3:].strip()
    # Remove headings markers but keep text
    md = re.sub(r'^#{1,6}\s+', '', md, flags=re.MULTILINE)
    # Remove tables
    md = re.sub(r'^\|.*\|$', '', md, flags=re.MULTILINE)
    md = re.sub(r'^[-|: ]+$', '', md, flags=re.MULTILINE)
    # Remove code blocks
    md = re.sub(r'```.*?```', '', md, flags=re.DOTALL)
    md = re.sub(r'`[^`]+`', '', md)
    # Remove images
    md = re.sub(r'!\[.*?\]\(.*?\)', '', md)
    # Remove links but keep text
    md = re.sub(r'\[([^\]]+)\]\([^\)]+\)', r'\1', md)
    # Remove bold/italic markers
    md = re.sub(r'\*{1,3}([^*]+)\*{1,3}', r'\1', md)
    md = re.sub(r'_{1,3}([^_]+)_{1,3}', r'\1', md)
    # Remove horizontal rules
    md = re.sub(r'^---+$', '', md, flags=re.MULTILINE)
    # Collapse blank lines
    md = re.sub(r'\n{3,}', '\n\n', md)
    return md.strip()

def to_podcast_script(title, body_text):
    """Use Claude to rewrite as a conversational podcast script."""
    client = anthropic.Anthropic(api_key=ANTHROPIC_API_KEY)
    prompt = f"""以下のブログ記事をポッドキャスト用のスクリプトに変換してください。

条件：
- 話し言葉・しゃべり口調（です/ます調でカジュアルに）
- 聴いてすぐわかる表現（数字や専門用語は言い換える）
- 自然な間投詞（「えー」「なので」「ちなみに」など）を適度に使う
- 表・数式・コードは読み上げ可能な文章に変換
- 全体を3〜8分の尺（1400〜2800文字程度）にまとめる
- オープニング：「こんにちは、ハマダです。今日は〜」で始める
- エンディング：「ということで、今日は〜でした。また次回！」で締める
- [NOTE: ...] や (読み飛ばし) などのメタ情報は含めない
- 純粋に読み上げられるテキストのみ出力

タイトル: {title}

記事本文:
{body_text[:4000]}
"""
    message = client.messages.create(
        model="claude-sonnet-4-6",
        max_tokens=3000,
        messages=[{"role": "user", "content": prompt}],
    )
    return message.content[0].text.strip()

def split_into_chunks(script, max_chars=200):
    """Split script into ~200 char chunks at sentence boundaries."""
    # Split at Japanese sentence endings
    sentences = re.split(r'(?<=[。！？\n])', script)
    chunks = []
    current = ""
    for s in sentences:
        s = s.strip()
        if not s:
            continue
        if len(current) + len(s) <= max_chars:
            current += s
        else:
            if current:
                chunks.append(current.strip())
            # If single sentence is longer than max_chars, split further
            if len(s) > max_chars:
                # Split at reading pauses (、 or mid-sentence)
                parts = re.split(r'(?<=[、,])', s)
                sub = ""
                for p in parts:
                    if len(sub) + len(p) <= max_chars:
                        sub += p
                    else:
                        if sub:
                            chunks.append(sub.strip())
                        sub = p
                if sub:
                    current = sub
                else:
                    current = ""
            else:
                current = s
    if current.strip():
        chunks.append(current.strip())
    return [c for c in chunks if c]

def tts_chunk(text, idx, out_dir):
    """Generate TTS for one chunk, save to out_dir/chunk_{idx:03d}.mp3"""
    url = f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE_ID}"
    headers = {
        "xi-api-key": ELEVENLABS_API_KEY,
        "Content-Type": "application/json",
        "Accept": "audio/mpeg",
    }
    payload = {
        "text": text,
        "model_id": MODEL_ID,
        "voice_settings": VOICE_SETTINGS,
    }
    for attempt in range(3):
        resp = requests.post(url, headers=headers, json=payload, timeout=30)
        if resp.status_code == 200:
            path = os.path.join(out_dir, f"chunk_{idx:03d}.mp3")
            with open(path, "wb") as f:
                f.write(resp.content)
            return path
        elif resp.status_code == 429:
            wait = 2 ** attempt
            print(f"  Rate limited, waiting {wait}s...")
            time.sleep(wait)
        else:
            print(f"  ElevenLabs error {resp.status_code}: {resp.text[:200]}")
            time.sleep(1)
    sys.exit(f"Failed to generate chunk {idx} after 3 attempts")

def concat_mp3s(chunk_paths, output_path):
    """Concatenate MP3 files using ffmpeg."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
        for p in chunk_paths:
            f.write(f"file '{p}'\n")
        list_file = f.name
    try:
        subprocess.run(
            ["ffmpeg", "-y", "-f", "concat", "-safe", "0",
             "-i", list_file, "-c", "copy", output_path],
            check=True, capture_output=True,
        )
    finally:
        os.unlink(list_file)

# ── Main ────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/gen_podcast.py <blog-slug>")
        print("Example: python3 scripts/gen_podcast.py 2026-05-01-bjj-best-sport-science")
        sys.exit(1)

    slug = sys.argv[1].replace(".md", "")

    load_env()
    global ELEVENLABS_API_KEY, ANTHROPIC_API_KEY
    ELEVENLABS_API_KEY = os.environ.get("ELEVENLABS_API_KEY", ELEVENLABS_API_KEY)
    ANTHROPIC_API_KEY  = os.environ.get("ANTHROPIC_API_KEY", ANTHROPIC_API_KEY)

    if not ELEVENLABS_API_KEY:
        sys.exit("ELEVENLABS_API_KEY not set")
    if not ANTHROPIC_API_KEY:
        sys.exit("ANTHROPIC_API_KEY not set")

    print(f"[1/5] Reading {slug}.md ...")
    raw = read_markdown(slug)
    title = extract_title(raw)
    body  = md_to_text(raw)
    print(f"      Title: {title}")
    print(f"      Body:  {len(body)} chars")

    print("[2/5] Converting to podcast script via Claude ...")
    script = to_podcast_script(title, body)
    print(f"      Script: {len(script)} chars")

    # Save script for review
    script_dir = os.path.join(os.path.dirname(__file__), "..", "public", "audio", "blog")
    os.makedirs(script_dir, exist_ok=True)
    script_path = os.path.join(script_dir, f"{slug}.txt")
    with open(script_path, "w", encoding="utf-8") as f:
        f.write(script)
    print(f"      Saved script → {script_path}")

    print("[3/5] Splitting into chunks ...")
    chunks = split_into_chunks(script, CHUNK_SIZE)
    print(f"      {len(chunks)} chunks")
    for i, c in enumerate(chunks):
        print(f"      [{i+1:02d}] {len(c)}chars: {c[:40]}...")

    print("[4/5] Generating TTS audio ...")
    with tempfile.TemporaryDirectory() as tmp:
        chunk_files = []
        for i, text in enumerate(chunks):
            print(f"      Chunk {i+1}/{len(chunks)}: {len(text)} chars", end=" ... ", flush=True)
            path = tts_chunk(text, i + 1, tmp)
            chunk_files.append(path)
            print("done")
            time.sleep(0.3)  # avoid rate limit

        print("[5/5] Concatenating MP3s ...")
        output_path = os.path.join(script_dir, f"{slug}.mp3")
        concat_mp3s(chunk_files, output_path)

    size_kb = os.path.getsize(output_path) // 1024
    print(f"\n✓ Done! → public/audio/blog/{slug}.mp3  ({size_kb} KB)")
    print(f"  URL:  /audio/blog/{slug}.mp3")

if __name__ == "__main__":
    main()
