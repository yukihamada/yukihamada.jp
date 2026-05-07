#!/usr/bin/env python3
"""
Tap, Tap, Tap — YouTube MP4 generator
音声タイミングずれ対策:
  - 音声は AAC 再エンコード(wav→aac)のみ、映像と完全同期
  - concat demuxer で静止画を正確な秒数でつなぐ
  - ASS 字幕で歌詞焼き込み(カラオケスタイル)
"""
import subprocess, os, json, sys

BASE = os.path.dirname(os.path.abspath(__file__))
IMG  = os.path.join(BASE, "assets/images/generated")
AUDIO = os.path.join(BASE, "assets/anthem.wav")
ASS   = "/tmp/tap_lyrics.ass"
CONCAT = "/tmp/tap_concat.txt"
OUTPUT = os.path.join(BASE, "tap_youtube.mp4")

W, H = 1920, 1080

# ── セクション: (name, start, end, image) ────────────────────────────────────
# endがNoneの場合は音声の末尾まで
SECTIONS = [
    ("intro",   0.0,   10.7,  "guitar_1.jpg"),
    ("pasha",   10.7,  19.6,  "pasha_v2_1.jpg"),
    ("pon",     19.6,  25.2,  "pon_v2_1.jpg"),
    ("kagi",    25.2,  34.9,  "kagi_v2_1.jpg"),
    ("pre",     34.9,  46.3,  "pre_1.jpg"),
    # コーラスは2枚に分割
    ("chorus1", 46.3,  57.15, "chorus_v2_1.jpg"),
    ("chorus2", 57.15, 68.0,  "chorus_v2_2.jpg"),
    ("jiuflow1",68.0,  73.5,  "jiuflow_v2_1.jpg"),
    ("jiuflow2",73.5,  79.0,  "jiuflow_v2_2.jpg"),
    ("elio1",   79.0,  84.2,  "elio_v2_1.jpg"),
    ("elio2",   84.2,  89.4,  "elio_v2_2.jpg"),
    ("soluna1", 89.4,  95.0,  "soluna_v2_1.jpg"),
    ("soluna2", 95.0,  100.6, "soluna_v2_2.jpg"),
    ("bridge",  100.6, 110.7, "bridge_v2_1.jpg"),
    # アウトロは2枚に分割
    ("outro1",  110.7, 137.8, "outro_v2_1.jpg"),
    ("outro2",  137.8, None,  "outro_v2_2.jpg"),
]

# ── 歌詞 (timestamp, text) ────────────────────────────────────────────────────
LYRICS = [
    (0.0,   "♪"),
    (0.8,   "俺が使うものは、俺が作る"),
    (10.7,  "2月、また来た。また胃が痛い。"),
    (13.8,  "でも今年は撮るだけ。パシャ。"),
    (16.5,  "もう終わり。"),
    (19.6,  "ハンコ？ 郵送？ 2025年ですよ。"),
    (22.9,  "ポン。それだけ。終わり。"),
    (25.2,  "深夜2時、「鍵が開きません」"),
    (27.8,  "布団から1秒、タップ。"),
    (30.0,  "「開きました」また寝る。"),
    (34.9,  "この歌、自転車の上で書いた。"),
    (39.5,  "声でメモ、信号待ちで続きを足した。"),
    (41.3,  "アイデアって 手が空いてるときに来ない。"),
    (43.3,  "だから声で捕まえる。"),
    (46.3,  "Tap, tap, tap"),
    (47.6,  "不便だと思ったら 作ればいい"),
    (51.4,  "Tap, tap, tap"),
    (52.8,  "作ったら みんなに渡せばいい"),
    (56.6,  "俺もやってる、君もやろう"),
    (59.3,  "面倒くさいの数だけ チャンスがある"),
    (61.3,  "一緒にやろうぜ"),
    (68.0,  "1年、絞められ続けた。"),
    (70.4,  "負けるパターン、全部同じだった。"),
    (73.3,  "フローチャートにして、アプリに入れた。"),
    (77.1,  "青帯。世界3位。"),
    (79.0,  "機内でも、無人島でも動く。"),
    (81.5,  "ネットなし、完全タダ、完全プライベート。"),
    (85.1,  "今は遅いけど、年内には超賢くなる。"),
    (89.4,  "10000台が同時に鳴る夜。"),
    (91.8,  "群衆が楽器になる。"),
    (95.0,  "1台は記憶、10000台はオーケストラ。"),
    (100.6, "バグ、あります。すぐ直します。"),
    (102.8, "高くしたくない。使ってほしいから。"),
    (104.6, "うまくいったら 一緒に得しよう。"),
    (110.7, "パシャで確定申告、終わらせて"),
    (112.5, "ポンで契約書、一秒で済ませて"),
    (114.8, "KAGIで家を、スマホで動かして"),
    (117.1, "Koeでアイデア、逃さないで"),
    (122.8, "JiuFlowで今日も、一段強くなる"),
    (128.1, "Elioに今日も、話しかけた"),
    (132.7, "Solunaで今夜も、フェスは続く"),
    (141.0, "♪"),
]

# ── ユーティリティ ────────────────────────────────────────────────────────────

def get_audio_duration(path):
    r = subprocess.run(
        ["ffprobe", "-v", "quiet", "-print_format", "json", "-show_format", path],
        capture_output=True, text=True, check=True
    )
    return float(json.loads(r.stdout)["format"]["duration"])

def ass_time(sec):
    """秒 → ASS タイムコード 0:00:00.00"""
    h = int(sec // 3600)
    m = int((sec % 3600) // 60)
    s = sec % 60
    cs = int(round((s - int(s)) * 100))
    if cs >= 100:
        cs = 99
    return f"{h}:{m:02d}:{int(s):02d}.{cs:02d}"

# ── ASS字幕生成 ───────────────────────────────────────────────────────────────

def make_ass(total_dur):
    """
    ASS字幕ファイルを生成
    色: ゴールド #D4A846 → ASS BGR = &H0046A8D4
    配置: 下中央 (Alignment=2)
    """
    header = """\
[Script Info]
ScriptType: v4.00+
PlayResX: 1920
PlayResY: 1080
WrapStyle: 0
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Lyric,Hiragino Sans,64,&H0046A8D4,&H00FFFFFF,&H00000000,&B2000000,-1,0,0,0,100,100,0,0,1,3,1,2,60,60,90,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text"""

    lines = [header]
    for i, (t, text) in enumerate(LYRICS):
        if text in ("♪",):
            continue
        # 次の歌詞が来るまで表示、最大 4.5 秒
        end_t = LYRICS[i + 1][0] if i + 1 < len(LYRICS) else total_dur
        end_t = min(end_t, t + 4.5)
        # ASS の Dialogue 行 — テキスト内のカンマはそのまま (10フィールド目以降が Text)
        lines.append(
            f"Dialogue: 0,{ass_time(t)},{ass_time(end_t)},Lyric,,0,0,0,,{text}"
        )

    with open(ASS, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    print(f"  ASS written: {ASS}")

# ── concat リスト生成 ─────────────────────────────────────────────────────────

def make_concat(total_dur):
    """
    ffmpeg concat demuxer 用テキストファイル生成
    各セクションに正確な duration を指定 → 音ズレなし
    """
    lines = []
    for name, start, end, img_file in SECTIONS:
        img_path = os.path.join(IMG, img_file)
        if not os.path.exists(img_path):
            print(f"  WARNING: 画像が見つかりません: {img_path}", file=sys.stderr)
        dur = (end if end is not None else total_dur) - start
        lines.append(f"file '{img_path}'")
        lines.append(f"duration {dur:.6f}")
    # concat demuxer の quirk: 最後のファイルを再度記載
    last_img = os.path.join(IMG, SECTIONS[-1][3])
    lines.append(f"file '{last_img}'")

    with open(CONCAT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"  Concat list written: {CONCAT}")

# ── ffmpeg 実行 ───────────────────────────────────────────────────────────────

def run_ffmpeg():
    """
    映像: concat(静止画スライドショー) + subtitles フィルタ(歌詞焼き込み)
    音声: WAV → AAC 192k (映像と同じ開始点、-shortest で長さを揃える)
    """
    # macOS の Hiragino フォントを fontsdir で指定
    fonts_dir = "/System/Library/Fonts"
    vf = (
        f"scale={W}:{H}:force_original_aspect_ratio=increase,"
        f"crop={W}:{H},"
        f"subtitles='{ASS}':fontsdir='{fonts_dir}'"
    )

    cmd = [
        "ffmpeg", "-y",
        # 映像: concat demuxer (duration で各画像の表示時間を制御)
        "-f", "concat", "-safe", "0", "-i", CONCAT,
        # 音声: anthem.wav
        "-i", AUDIO,
        # フィルタ: スケール + 歌詞焼き込み + fps
        "-vf", vf + ",fps=30",
        # 映像コーデック
        "-c:v", "libx264", "-preset", "slow", "-crf", "20", "-pix_fmt", "yuv420p",
        # 音声コーデック (高品質 AAC)
        "-c:a", "aac", "-b:a", "192k",
        # 映像 or 音声の短い方に合わせる
        "-shortest",
        # YouTube 向け: fast start (moov atom を先頭に)
        "-movflags", "+faststart",
        OUTPUT,
    ]

    print("\nffmpeg 実行中...")
    print("  " + " ".join(cmd[:8]) + " ...")
    result = subprocess.run(cmd, capture_output=False)
    if result.returncode != 0:
        print("ERROR: ffmpeg が失敗しました", file=sys.stderr)
        sys.exit(1)

# ── メイン ────────────────────────────────────────────────────────────────────

def main():
    print(f"[1/4] 音声の長さを取得...")
    dur = get_audio_duration(AUDIO)
    print(f"       {dur:.2f}s ({dur/60:.1f}分)")

    print(f"[2/4] ASS 字幕生成...")
    make_ass(dur)

    print(f"[3/4] concat リスト生成...")
    make_concat(dur)

    print(f"[4/4] ffmpeg で動画生成...")
    run_ffmpeg()

    size_mb = os.path.getsize(OUTPUT) / 1024 / 1024
    print(f"\n完了! {OUTPUT}")
    print(f"      ファイルサイズ: {size_mb:.1f} MB")
    print(f"      解像度: {W}x{H} / コーデック: H.264 / 音声: AAC 192k")

if __name__ == "__main__":
    main()
