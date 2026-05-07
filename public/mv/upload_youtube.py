#!/usr/bin/env python3
"""
全MV動画をYouTubeにアップロード（バズ最適化済み）

初回のみ: ブラウザでGoogleログインが必要
  ! python3 upload_youtube.py --auth

通常: 全動画アップロード
  python3 upload_youtube.py
"""
import os, sys, json, pickle, time

BASE = os.path.dirname(os.path.abspath(__file__))
TOKEN_FILE = os.path.join(BASE, ".youtube_token.pickle")

# ═══════════════════════════════════════════════════════════════════
# 動画メタデータ — YouTube バズ最適化
# ═══════════════════════════════════════════════════════════════════
VIDEOS = [
    {
        "file": "tap_youtube.mp4",
        "title": "【MV】Tap, Tap, Tap — 不便だと思ったら作ればいい｜全部自分で作った男の歌",
        "description": """確定申告3秒、電子契約1タップ、スマートロック、音声メモ、BJJ分析、ローカルAI、フェスデバイス——全部自分で作った。

「面倒くさい」の数だけチャンスがある。
俺もやってる、君もやろう。

▼ 登場するアプリ（全部無料）
📸 パシャ（確定申告）: https://pasha.run
📝 ポン（電子契約）: https://pasha.run/pon
🔑 KAGI（スマートホーム）: https://pasha.run/kacha
🎙 Koe（音声入力）: https://app.koe.live
🥋 JiuFlow（BJJ分析）: https://jiuflow.art
🤖 Elio（ローカルAI）: https://elio.love
🎵 SOLUNA（フェスデバイス）: https://solun.art

▼ チャプター
0:00 イントロ — 俺が使うものは、俺が作る
0:10 パシャ — 確定申告を3秒で
0:19 ポン — ハンコ不要の電子契約
0:25 KAGI — 深夜2時のスマートロック
0:34 Koe — 自転車で書いた歌
0:46 サビ — Tap, tap, tap
1:08 JiuFlow — 青帯・世界3位への道
1:19 Elio — 機内でも動くローカルAI
1:29 SOLUNA — 10000台が同時に鳴る夜
1:40 ブリッジ — バグあります、すぐ直します
1:50 アウトロ — 全アプリまとめ

#インディーデベロッパー #個人開発 #確定申告 #アプリ開発 #エンジニア #プログラミング #AI #スマートホーム #BJJ #柔術 #音楽 #MV #自作アプリ #テック #startup""",
        "tags": ["インディーデベロッパー","個人開発","確定申告","アプリ開発","エンジニア","プログラミング","AI","スマートホーム","BJJ","柔術","MV","自作アプリ","テック","startup","パシャ","ポン","KAGI","Koe","JiuFlow","Elio","SOLUNA","濱田優貴"],
        "category": "10",  # Music
    },
    {
        "file": "jiujitsu_youtube.mp4",
        "title": "【MV】それ恋じゃなくて柔術｜青帯・世界3位のBJJアンセム🥋",
        "description": """柔術にハマった男の歌。1年半で青帯、ワールドマスター3位。

「それ恋じゃない、柔術だ」

▼ JiuFlow（BJJ分析アプリ）
🥋 https://jiuflow.art
10のフローチャートで最短勝利。試合分析・ゲームプラン作成。

#BJJ #柔術 #ブラジリアン柔術 #青帯 #ワールドマスター #MV #格闘技 #JiuFlow #grappling #jiujitsu""",
        "tags": ["BJJ","柔術","ブラジリアン柔術","青帯","ワールドマスター","MV","格闘技","JiuFlow","grappling","jiujitsu","武道","martial arts"],
        "category": "10",
    },
    {
        "file": "hack_youtube.mp4",
        "title": "【MV】Hack, Hack, Hack — コードで世界を変える🔥",
        "description": """プログラマーのアンセム。ハックし続ける者たちへ。

#プログラミング #エンジニア #ハック #コーディング #MV #テック #developer #coding #hack""",
        "tags": ["プログラミング","エンジニア","ハック","コーディング","MV","テック","developer","coding","hack","startup"],
        "category": "10",
    },
    {
        "file": "attention_youtube.mp4",
        "title": "【MV】I Need Your Attention — 注目してくれ、俺のアプリを",
        "description": """個人開発者の叫び。作ったものを世界に届けたい。

#個人開発 #インディーデベロッパー #MV #attention #アプリ開発 #startup""",
        "tags": ["個人開発","インディーデベロッパー","MV","attention","アプリ開発","startup","エンジニア"],
        "category": "10",
    },
    {
        "file": "musubinaosu_youtube.mp4",
        "title": "【MV】結び直す朝 — 壊れても、また結べばいい",
        "description": """朝の再出発の歌。バグがあっても、失敗しても、結び直せばいい。

#朝活 #再出発 #MV #日本語 #インディーズ #個人開発""",
        "tags": ["朝活","再出発","MV","日本語","インディーズ","個人開発","結び直す"],
        "category": "10",
    },
    {
        "file": "claude-code_youtube.mp4",
        "title": "【MV】Claude Code Anthem — AIと一緒にコードを書く時代🤖",
        "description": """Claude Codeへのラブレター。AIペアプログラミングの歌。

AIと人間が一緒にコードを書く。それが2025年のプログラミング。

🧠 teai.io: https://teai.io
🤖 Elio: https://elio.love

#ClaudeCode #AI #プログラミング #ペアプログラミング #Anthropic #MV #エンジニア #コーディング""",
        "tags": ["ClaudeCode","AI","プログラミング","ペアプログラミング","Anthropic","MV","エンジニア","コーディング","Claude","LLM"],
        "category": "10",
    },
    {
        "file": "local-ai_youtube.mp4",
        "title": "【MV】Local AI — ネットなし完全無料のAIが来る",
        "description": """ローカルで動くAIの歌。インターネット不要、完全無料、完全プライベート。

🤖 Elio（ローカルAI）: https://elio.love

#ローカルAI #オフラインAI #プライバシー #AI #MV #Elio #エッジAI""",
        "tags": ["ローカルAI","オフラインAI","プライバシー","AI","MV","Elio","エッジAI","local AI","privacy"],
        "category": "10",
    },
    {
        "file": "kagi_youtube.mp4",
        "title": "【MV】KAGI — 深夜2時、布団から鍵を開ける🔑",
        "description": """スマートロックの歌。深夜2時の「鍵が開きません」を布団から1秒で解決。

🔑 KAGI: https://pasha.run/kacha

#スマートロック #スマートホーム #IoT #MV #KAGI #Airbnb #民泊""",
        "tags": ["スマートロック","スマートホーム","IoT","MV","KAGI","Airbnb","民泊","smart home","smart lock"],
        "category": "10",
    },
    {
        "file": "koe_youtube.mp4",
        "title": "【MV】Koe — 声でアイデアを捕まえる🎙",
        "description": """音声入力アプリKoeの歌。自転車の上でも、走りながらでも、声でテキスト化。

🎙 Koe: https://app.koe.live

#音声入力 #Koe #ボイスメモ #MV #生産性 #アプリ""",
        "tags": ["音声入力","Koe","ボイスメモ","MV","生産性","アプリ","voice input","productivity"],
        "category": "10",
    },
]

def get_credentials():
    """YouTube API認証"""
    from google_auth_oauthlib.flow import InstalledAppFlow
    from google.auth.transport.requests import Request

    SCOPES = ["https://www.googleapis.com/auth/youtube", "https://www.googleapis.com/auth/youtube.upload"]
    creds = None

    if os.path.exists(TOKEN_FILE):
        with open(TOKEN_FILE, "rb") as f:
            creds = pickle.load(f)

    if creds and creds.expired and creds.refresh_token:
        creds.refresh(Request())
    elif not creds or not creds.valid:
        # client_secret.json が必要
        secret_paths = [
            os.path.join(BASE, "client_secret.json"),
            os.path.expanduser("~/client_secret.json"),
            os.path.expanduser("~/Downloads/client_secret.json"),
        ]
        secret_file = None
        for p in secret_paths:
            if os.path.exists(p):
                secret_file = p
                break

        if not secret_file:
            print("="*60)
            print("YouTube API認証にはOAuth Client IDが必要です。")
            print()
            print("1. https://console.cloud.google.com/apis/credentials")
            print("   → 「認証情報を作成」→「OAuthクライアントID」")
            print("   → アプリケーションの種類: 「デスクトップアプリ」")
            print("   → 作成 → JSONをダウンロード")
            print()
            print("2. ダウンロードしたファイルを以下のいずれかに配置:")
            print(f"   {os.path.join(BASE, 'client_secret.json')}")
            print(f"   ~/client_secret.json")
            print(f"   ~/Downloads/client_secret.json")
            print()
            print("3. 再度このスクリプトを実行")
            print("="*60)
            sys.exit(1)

        flow = InstalledAppFlow.from_client_secrets_file(secret_file, SCOPES)
        creds = flow.run_local_server(port=8080)

        with open(TOKEN_FILE, "wb") as f:
            pickle.dump(creds, f)
        print("認証完了! トークンを保存しました。")

    return creds

def upload_video(youtube, video_info):
    """1本の動画をアップロード"""
    from googleapiclient.http import MediaFileUpload

    filepath = os.path.join(BASE, video_info["file"])
    if not os.path.exists(filepath):
        print(f"  SKIP: {video_info['file']} not found")
        return None

    size_mb = os.path.getsize(filepath) / 1024 / 1024
    print(f"\n  Uploading: {video_info['file']} ({size_mb:.1f} MB)")
    print(f"  Title: {video_info['title']}")

    body = {
        "snippet": {
            "title": video_info["title"],
            "description": video_info["description"],
            "tags": video_info["tags"],
            "categoryId": video_info["category"],
            "defaultLanguage": "ja",
            "defaultAudioLanguage": "ja",
        },
        "status": {
            "privacyStatus": "public",
            "selfDeclaredMadeForKids": False,
            "embeddable": True,
            "publicStatsViewable": True,
        },
    }

    media = MediaFileUpload(
        filepath,
        mimetype="video/mp4",
        resumable=True,
        chunksize=10 * 1024 * 1024,  # 10MB chunks
    )

    request = youtube.videos().insert(
        part="snippet,status",
        body=body,
        media_body=media,
    )

    response = None
    while response is None:
        status, response = request.next_chunk()
        if status:
            pct = int(status.progress() * 100)
            print(f"\r  uploading... {pct}%", end="", flush=True)

    video_id = response["id"]
    print(f"\n  DONE: https://youtube.com/watch?v={video_id}")
    return video_id

def main():
    from googleapiclient.discovery import build

    if "--auth" in sys.argv:
        get_credentials()
        print("認証OK!")
        return

    creds = get_credentials()
    youtube = build("youtube", "v3", credentials=creds)

    uploaded = []
    for v in VIDEOS:
        try:
            vid = upload_video(youtube, v)
            if vid:
                uploaded.append((v["title"], vid))
        except Exception as e:
            print(f"  ERROR: {e}")

    print(f"\n{'='*60}")
    print(f"アップロード完了: {len(uploaded)}/{len(VIDEOS)} 本")
    for title, vid in uploaded:
        print(f"  {title}")
        print(f"    https://youtube.com/watch?v={vid}")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
