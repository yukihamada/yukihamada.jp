#!/usr/bin/env python3
"""
tap.html をブラウザで再生し、画面録画 → 元音声と合成して YouTube 用 mp4 を出力。

YouTube向け: 再生ボタンのスプラッシュ画面はカット。
曲が始まる瞬間から動画がスタートするようにする。

方式:
  1. Playwright でページを開き、再生ボタンをクリック（録画前）
  2. スプラッシュがフェードアウトするのを待つ
  3. 録画用の新しいコンテキストで、再生中のページをスクリーンショット連写
     → ではなく、録画コンテキストで開いて即座にJSで再生開始
  4. ffmpeg で元音声と合成
"""
import subprocess, os, sys, time, json

BASE = os.path.dirname(os.path.abspath(__file__))
AUDIO = os.path.join(BASE, "assets/anthem.wav")
VIDEO_RAW = "/tmp/tap_raw_recording.webm"
OUTPUT = os.path.join(BASE, "tap_youtube.mp4")

W, H = 1920, 1080

def get_audio_duration(path):
    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_format", path],
        capture_output=True, text=True, check=True
    )
    return float(json.loads(r.stdout)["format"]["duration"])

def record_browser():
    """Playwright でブラウザ録画 — スプラッシュなし"""
    from playwright.sync_api import sync_playwright

    audio_dur = get_audio_duration(AUDIO)

    print(f"[1/3] ブラウザ録画開始 (Chromium {W}x{H}, {audio_dur:.1f}s)")

    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless=False,
            args=[
                "--autoplay-policy=no-user-gesture-required",
                "--use-fake-ui-for-media-stream",
                "--use-fake-device-for-media-stream",
                "--window-size=1920,1080",
                "--start-maximized",
            ],
        )

        # 録画コンテキスト
        context = browser.new_context(
            viewport={"width": W, "height": H},
            device_scale_factor=1,
            record_video_dir="/tmp/tap_video/",
            record_video_size={"width": W, "height": H},
        )
        page = context.new_page()

        # 録画はコンテキスト作成時に開始される → ここが録画の0秒
        rec_start = time.time()

        # ページ読み込み
        tap_url = f"file://{os.path.join(BASE, 'tap.html')}"
        page.goto(tap_url, wait_until="networkidle")
        load_time = time.time() - rec_start
        print(f"  ページ読み込み完了 ({load_time:.1f}s)")

        # JSでスプラッシュを非表示 + 再生開始
        # audio.play() が失敗しても手動でアニメーションを駆動
        page.evaluate("""() => {
            // スプラッシュ消去
            const ps = document.getElementById('play-screen');
            if (ps) { ps.style.display = 'none'; }

            // init
            if (typeof init === 'function') init();

            // startPlay() を試みる
            try { startPlay(); } catch(e) { console.log('startPlay error:', e); }

            // フォールバック: audio.play() が失敗した場合に手動駆動
            const a = document.getElementById('audio');
            const playPromise = a.play();
            if (playPromise) {
                playPromise.catch(() => {
                    console.log('audio.play() failed, using manual driver');
                });
            }

            // 確実にisPlayingをtrueにしてaudioLoopを起動
            setTimeout(() => {
                isPlaying = true;
                // audio要素の currentTime を手動でインクリメントするタイマー
                // (ヘッドレスで音声再生できない場合のフォールバック)
                if (a.paused) {
                    console.log('Audio is paused, starting manual time driver');
                    const startMs = performance.now();
                    const driver = setInterval(() => {
                        const elapsed = (performance.now() - startMs) / 1000;
                        a.currentTime = elapsed;
                        if (elapsed >= a.duration) clearInterval(driver);
                    }, 40); // 25fps相当
                }
                requestAnimationFrame(audioLoop);
            }, 100);
        }""")

        # startPlay() が呼ばれた正確なタイムスタンプを記録
        play_start = time.time()
        trim_offset = play_start - rec_start
        print(f"  曲を再生開始 (録画開始から {trim_offset:.2f}s 後)")

        # このオフセットをファイルに保存（merge時に使う）
        with open("/tmp/tap_trim_offset.txt", "w") as f:
            f.write(str(trim_offset))

        # 音声の長さだけ待つ（play_startから計測）
        target = audio_dur + 2
        while time.time() - play_start < target:
            elapsed = time.time() - play_start
            pct = min(100, elapsed / audio_dur * 100)
            mins = int(elapsed) // 60
            secs = int(elapsed) % 60
            print(f"\r  録画中... {mins}:{secs:02d} / {int(audio_dur)//60}:{int(audio_dur)%60:02d} ({pct:.0f}%)", end="", flush=True)
            time.sleep(1)
        print()

        # 録画停止
        video_path = page.video.path()
        context.close()
        browser.close()

    # 録画ファイルを移動
    if os.path.exists(video_path):
        subprocess.run(["mv", video_path, VIDEO_RAW], check=True)
        size_mb = os.path.getsize(VIDEO_RAW) / 1024 / 1024
        print(f"  録画完了: {VIDEO_RAW} ({size_mb:.1f} MB)")
    else:
        print(f"ERROR: 録画ファイルが見つかりません: {video_path}", file=sys.stderr)
        sys.exit(1)

    subprocess.run(["rm", "-rf", "/tmp/tap_video/"], check=False)

def merge_video_audio():
    """録画映像の先頭をトリム + 元音声と合成 → MP4"""
    with open("/tmp/tap_trim_offset.txt") as f:
        trim = float(f.read().strip())
    print(f"\n[2/3] 映像 + 音声を合成 (先頭 {trim}s トリム)...")

    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_streams", VIDEO_RAW],
        capture_output=True, text=True, check=True
    )
    streams = json.loads(r.stdout)["streams"]
    for s in streams:
        print(f"  {s['codec_type']}: {s.get('width','?')}x{s.get('height','?')} {s['codec_name']} dur={s.get('duration','?')}")

    cmd = [
        "ffmpeg", "-y",
        # 映像入力 — 先頭をトリムして曲開始に合わせる
        "-ss", str(trim), "-i", VIDEO_RAW,
        # 音声入力
        "-i", AUDIO,
        # H.264 / YouTube 最適
        "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-pix_fmt", "yuv420p",
        # AAC 192k
        "-c:a", "aac", "-b:a", "192k",
        # 映像のみ + 音声のみ
        "-map", "0:v:0", "-map", "1:a:0",
        # 音声の長さに合わせて切る
        "-shortest",
        # YouTube 最適化
        "-movflags", "+faststart",
        OUTPUT,
    ]

    print("  ffmpeg 実行中...")
    result = subprocess.run(cmd, capture_output=False)
    if result.returncode != 0:
        print("ERROR: ffmpeg 合成に失敗", file=sys.stderr)
        sys.exit(1)

def verify():
    """出力動画を検証"""
    print(f"\n[3/3] 検証...")
    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_streams", "-show_format", OUTPUT],
        capture_output=True, text=True, check=True
    )
    data = json.loads(r.stdout)
    for s in data["streams"]:
        ct = s["codec_type"]
        if ct == "video":
            print(f"  映像: {s['width']}x{s['height']} {s['codec_name']} {s.get('r_frame_rate','')} {s.get('nb_frames','')} frames")
        elif ct == "audio":
            print(f"  音声: {s['codec_name']} {s.get('sample_rate','')}Hz {s.get('bit_rate','?')}bps")

    dur = float(data["format"]["duration"])
    size_mb = float(data["format"]["size"]) / 1024 / 1024
    print(f"  再生時間: {int(dur)//60}:{int(dur)%60:02d}")
    print(f"  ファイルサイズ: {size_mb:.1f} MB")
    print(f"\n完了! {OUTPUT}")

def main():
    record_browser()
    merge_video_audio()
    verify()

if __name__ == "__main__":
    main()
