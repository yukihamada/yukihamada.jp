#!/usr/bin/env python3
"""
全MV HTMLページをブラウザ録画 → 元音声と合成して YouTube 用 mp4 を出力。
"""
import subprocess, os, sys, time, json

BASE = os.path.dirname(os.path.abspath(__file__))
W, H = 1920, 1080

# (html, audio, overlay_id, start_js)
PAGES = [
    ("tap.html",        "assets/anthem.wav",             "play-screen", "startPlay()"),
    ("jiujitsu.html",   "assets/jiujitsu.mp3",           "start-overlay", "startPlay()"),
    ("hack.html",       "assets/hack.wav",               "start-overlay", "startExperience()"),
    ("attention.html",  "assets/attention.mp3",           "start-overlay", "startPlay()"),
    ("musubinaosu.html","assets/musubinaosu.mp3",         "st",           "go()"),
    ("claude-code.html","assets/claude_code_anthem.wav",  "start",        "go()"),
    ("local-ai.html",   "assets/local-ai/track_en.mp3",  "start",        "go()"),
    ("kagi.html",       "kagi.mp3",                       "st",           "go()"),
    ("koe.html",        "koe_song.mp3",                   None,           "__KOE_SPECIAL__"),
]

def get_duration(path):
    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_format", path],
        capture_output=True, text=True, check=True
    )
    return float(json.loads(r.stdout)["format"]["duration"])

def record_one(html_file, audio_file, overlay_id, start_js):
    from playwright.sync_api import sync_playwright

    name = html_file.replace(".html", "")
    audio_path = os.path.join(BASE, audio_file)
    output_path = os.path.join(BASE, f"{name}_youtube.mp4")
    raw_path = f"/tmp/tap_raw_{name}.webm"
    video_dir = f"/tmp/tap_video_{name}/"

    if not os.path.exists(audio_path):
        print(f"  SKIP: {audio_file} not found")
        return

    audio_dur = get_duration(audio_path)
    print(f"\n{'='*60}")
    print(f"  {name} ({audio_dur:.0f}s / {audio_dur/60:.1f}min)")
    print(f"{'='*60}")

    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=False,
            args=[
                "--autoplay-policy=no-user-gesture-required",
                "--use-fake-ui-for-media-stream",
                "--use-fake-device-for-media-stream",
                "--window-size=1920,1080",
            ],
        )
        context = browser.new_context(
            viewport={"width": W, "height": H},
            device_scale_factor=1,
            record_video_dir=video_dir,
            record_video_size={"width": W, "height": H},
        )
        page = context.new_page()
        rec_start = time.time()

        url = f"file://{os.path.join(BASE, html_file)}"
        page.goto(url, wait_until="networkidle")
        load_time = time.time() - rec_start
        print(f"  loaded ({load_time:.1f}s)")

        # テキストを大きくするCSSを注入
        page.add_style_tag(content="""
            body { zoom: 1.3; }
        """)

        # オーバーレイを消して再生開始
        hide_js = ""
        if overlay_id:
            hide_js = f"var ov=document.getElementById('{overlay_id}'); if(ov)ov.style.display='none';"

        if start_js == "__KOE_SPECIAL__":
            # Koe専用: audio要素を完全にフェイクして手動駆動
            page.evaluate("""() => {
                // play-btnを隠す
                var btn = document.getElementById('play-btn');
                if(btn) btn.style.display = 'none';

                // audio要素の代わりにフェイクオブジェクトで上書き
                var realAudio = document.getElementById('audio');
                var fakeTime = 0;
                var fakeDuration = 152; // koe_song.mp3 の長さ
                var startMs = performance.now();

                // グローバルのaudio変数を上書き
                window.audio = {
                    get currentTime() { return (performance.now() - startMs) / 1000; },
                    set currentTime(v) { /* ignore */ },
                    get duration() { return fakeDuration; },
                    get paused() { return false; },
                    pause: function() {},
                    play: function() { return Promise.resolve(); },
                    addEventListener: function() {},
                };

                // tick()を起動
                lastTime = performance.now();
                cur = -1;
                requestAnimationFrame(tick);
            }""")
        else:
            page.evaluate(f"""() => {{
                {hide_js}
                // 全シーンの最初をアクティブに
                var sc = document.querySelector('.scene');
                if(sc) sc.classList.add('active');
                // 再生開始
                try {{ {start_js} }} catch(e) {{ console.log('start error:', e); }}
                // フォールバック: 手動タイム駆動
                setTimeout(() => {{
                    var a = document.querySelector('audio') || document.getElementById('audio') || window.au || window.audio;
                    if (a && a.paused) {{
                        console.log('Manual time driver for ' + '{name}');
                        if (typeof isPlaying !== 'undefined') isPlaying = true;
                        if (typeof on !== 'undefined') on = true;
                        if (typeof playing !== 'undefined') playing = true;
                        var st = performance.now();
                        var drv = setInterval(() => {{
                            var el = (performance.now() - st) / 1000;
                            try {{ a.currentTime = el; }} catch(e) {{}}
                            if (el >= (a.duration||999)) clearInterval(drv);
                        }}, 40);
                        if (typeof audioLoop === 'function') requestAnimationFrame(audioLoop);
                        if (typeof lp === 'function') requestAnimationFrame(lp);
                        if (typeof loop === 'function') requestAnimationFrame(loop);
                    }}
                }}, 200);
            }}""")

        play_start = time.time()
        trim = play_start - rec_start
        print(f"  playing (trim={trim:.1f}s)")

        target = audio_dur + 2
        while time.time() - play_start < target:
            elapsed = time.time() - play_start
            pct = min(100, elapsed / audio_dur * 100)
            m, s = int(elapsed) // 60, int(elapsed) % 60
            print(f"\r  recording {m}:{s:02d}/{int(audio_dur)//60}:{int(audio_dur)%60:02d} ({pct:.0f}%)", end="", flush=True)
            time.sleep(2)
        print()

        video_path = page.video.path()
        context.close()
        browser.close()

    if not os.path.exists(video_path):
        print(f"  ERROR: no recording")
        return

    subprocess.run(["mv", video_path, raw_path], check=True)
    subprocess.run(["rm", "-rf", video_dir], check=False)

    # merge
    print(f"  merging (trim {trim:.1f}s)...")
    cmd = [
        "ffmpeg", "-y",
        "-ss", str(trim), "-i", raw_path,
        "-i", audio_path,
        "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-pix_fmt", "yuv420p",
        "-c:a", "aac", "-b:a", "192k",
        "-map", "0:v:0", "-map", "1:a:0",
        "-shortest", "-movflags", "+faststart",
        output_path,
    ]
    r = subprocess.run(cmd, capture_output=True)
    if r.returncode != 0:
        print(f"  ERROR: ffmpeg failed")
        print(r.stderr.decode()[-500:])
        return

    size_mb = os.path.getsize(output_path) / 1024 / 1024
    print(f"  DONE: {output_path} ({size_mb:.1f} MB)")

def main():
    # 引数で特定のページだけ指定可能
    targets = sys.argv[1:] if len(sys.argv) > 1 else [p[0] for p in PAGES]

    for html, audio, overlay, js in PAGES:
        if html in targets or html.replace(".html", "") in targets:
            record_one(html, audio, overlay, js)

if __name__ == "__main__":
    main()
