#!/usr/bin/env python3
"""
YouTube動画の説明文を更新（歌詞+他の曲へのリンク）
"""
import os, sys, pickle
BASE = os.path.dirname(os.path.abspath(__file__))
TOKEN_FILE = os.path.join(BASE, ".youtube_token.pickle")

# 全動画ID
IDS = {
    "tap":         "H4P6COnpwr4",
    "jiujitsu":    "nLAZbpIPHsQ",
    "hack":        "daK2mItxRB4",
    "attention":   "RcDI7z60NPI",
    "musubinaosu": "RnmxBdDmxfU",
    "claude-code": "buB-x0Sw6qk",
    "local-ai":    "HwvRG0fQnt0",
    "kagi":        "YPJah03fNW4",
    "koe":         "xCGIy6vThbk",
}

OTHER_SONGS = """
━━━━━━━━━━━━━━━━━━
他の曲も聴いてね
━━━━━━━━━━━━━━━━━━
{links}

━━━━━━━━━━━━━━━━━━
Music & Lyrics: 濱田優貴 / Yuki Hamada
All songs generated with AI (Suno)
All visuals generated with AI (Gemini)
All code written with AI (Claude Code)

Web: https://yukihamada.jp/mv/
"""

SONG_TITLES = {
    "tap":         "Tap, Tap, Tap — 不便だと思ったら作ればいい",
    "jiujitsu":    "それ恋じゃなくて柔術 — BJJアンセム",
    "hack":        "Hack, Hack, Hack — コードで世界を変える",
    "attention":   "I Need Your Attention — アテンションください",
    "musubinaosu": "結び直す朝 — 壊れても、また結べばいい",
    "claude-code": "Claude Code Anthem — AIと一緒にコードを書く",
    "local-ai":    "Local AI — ネットなし完全無料のAI",
    "kagi":        "KAGI — 深夜2時、布団から鍵を開ける",
    "koe":         "Koe — 声でアイデアを捕まえる",
}

VIDEOS = [
    {
        "id": IDS["tap"],
        "key": "tap",
        "title": "【MV】Tap, Tap, Tap — 不便だと思ったら作ればいい｜全部自分で作った男の歌",
        "description": """「面倒くさい」の数だけチャンスがある。
俺もやってる、君もやろう。

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

▼ 歌詞
俺が使うものは、俺が作る

2月、また来た。また胃が痛い。
でも今年は撮るだけ。パシャ。
もう終わり。

ハンコ？ 郵送？ 2025年ですよ。
ポン。それだけ。終わり。

深夜2時、「鍵が開きません」
布団から1秒、タップ。
「開きました」また寝る。

この歌、自転車の上で書いた。
声でメモ、信号待ちで続きを足した。
アイデアって 手が空いてるときに来ない。
だから声で捕まえる。

Tap, tap, tap
不便だと思ったら 作ればいい
Tap, tap, tap
作ったら みんなに渡せばいい
俺もやってる、君もやろう
面倒くさいの数だけ チャンスがある
一緒にやろうぜ

1年、絞められ続けた。
負けるパターン、全部同じだった。
フローチャートにして、アプリに入れた。
青帯。世界3位。

機内でも、無人島でも動く。
ネットなし、完全タダ、完全プライベート。
今は遅いけど、年内には超賢くなる。

10000台が同時に鳴る夜。
群衆が楽器になる。
1台は記憶、10000台はオーケストラ。

バグ、あります。すぐ直します。
高くしたくない。使ってほしいから。
うまくいったら 一緒に得しよう。

パシャで確定申告、終わらせて
ポンで契約書、一秒で済ませて
KAGIで家を、スマホで動かして
Koeでアイデア、逃さないで
JiuFlowで今日も、一段強くなる
Elioに今日も、話しかけた
Solunaで今夜も、フェスは続く""",
        "tags": ["インディーデベロッパー","個人開発","MV","自作アプリ","エンジニア","Tap","確定申告","スマートホーム","BJJ","AI","濱田優貴"],
    },
    {
        "id": IDS["jiujitsu"],
        "key": "jiujitsu",
        "title": "【MV】それ恋じゃなくて柔術｜青帯・世界3位のBJJアンセム🥋",
        "description": """柔術にハマった男の歌。恋の言葉と柔術の用語がリンクする。

▼ 歌詞
また胸の上で 息が詰まる
離れたくない でも正直ちょっと苦しい
手首つかまれて 逃げ道ない
なのに笑ってる 私もおかしい

夜更けのメモにしがみついて
「会いたい」より増えていく数字
1・2・3… 深く吸って
目を開けたら 世界がスローになる

依存じゃない、ポジションだ
離れられないのはガードが固いから
愛が苦しい？ それ三角絞め
恋に落ちた？ いやハーフのまんま
壊されたい？ ううん崩されたいだけ
ベースと重心で優しく

朝のベランダ 風に揺れる白い君
袖口のほつれさえも愛しい
カードの磁気が擦り切れるほど通って
今日の痛みは 明日の私を守る

弱さも全部持ち寄って
絡んで ほどいて また組み直す
涙じゃなくて汗と笑い

フレーム作って呼吸を守る
執着じゃない、ディフェンスだ
距離を置くのはスペース作るため
「もう無理かも」？ それは肩固め
君に縛られた？ いや襟を握っただけ

青い日々 紫の夢
焦らず一つずつスイープしていく
世界が狭くなるほど自由になる

右手でフレーム 左で襟
腰切って 回って 返して 立ち上がる
抑え込み 逃げて また笑う
「大丈夫、呼吸して」——それだけで強くなる

離れられないのは"好き"だけじゃない
愛が欲しい？ うん、でもまずは一本""",
        "tags": ["BJJ","柔術","ブラジリアン柔術","青帯","MV","格闘技","三角絞め","ガード","スイープ","JiuFlow","恋愛","ラブソング"],
    },
    {
        "id": IDS["hack"],
        "key": "hack",
        "title": "【MV】Hack, Hack, Hack — ソフトの次はハードを作る🔥",
        "description": """ソフトウェアだけじゃ足りなかった。次は手で触れるものを作る。

▼ 歌詞
俺が使うものは、俺が作る
ソフトだけじゃ、足りなかった
次は、手で触れるものを作る

パシャ・ポン・KAGI・Koe
鍵・声・情報で記録
全部タップひとつ。全部、俺が作った。
ハード、やる。

でもある日、気づいた。スマホを出せない時——
アプリは使えない。
走りながら。荷物持ちながら。手が塞がってる時——
声しか使えない。

だから次は、形にする。
500円玉サイズのデバイスを作ってる。
耳に刺さる。首から下げる。腕に巻く。
喋るだけでテキストになる。
Wi-Fiなし。スマホなし。
ポケットの中で動く。
声が、一番速いインターフェースだ。

同じチップに、別のファームを焼いたら
フェスのスピーカーになった。
10,000個が同時に鳴る。誤差1ミリ秒。
群衆が楽器になる夜を、作ってる。

眠ってる間も、4台のAIが動いてる。
Hachi、Kuro、Ichi、Ni。
コードを書いて、テストして、デプロイして。
朝起きたら、昨日の機能が完成してた。
俺の分身が、24時間止まらない。""",
        "tags": ["プログラミング","エンジニア","ハードウェア","IoT","コーディング","MV","テック","developer","Koe","SOLUNA","AI"],
    },
    {
        "id": IDS["attention"],
        "key": "attention",
        "title": "【MV】I Need Your Attention — アテンションください💡",
        "description": """脳科学×恋愛のMV。ニューロン、自由エネルギー原理、アテンション機構が恋のメタファーになる。

▼ 歌詞
まぶたの裏で 光が灯る
星座みたいに ニューロンが瞬く
「いっしょに発火したら結びつく」
ヘッブ則みたいな はじまりの予感

君の名前で 世界は予測を始める
ズレを数えて 胸が熱くなる
エラーを 最小化しよう
言葉より先に 手を繋いで

深呼吸して 自由エネルギー
そっと下げたら 合図をちょうだい

アテンションください
アテンションください
ウェイトを上げて 閾値を超えたらキス

君の笑顔を埋め込みに変えて
胸の潜在空間で何度も再生
夜の損失は 逆伝播でほどける
涙の勾配で 心を最適化

ドーパミンはちいさな予測誤差の花火
「嬉しい」の確率 跳ね上がるよ
触れた温度で シナプス可塑性
LTPみたいに 記憶は強く

余計なノイズはドロップアウト
君の声だけ 残していこう
アテンションください""",
        "tags": ["アテンション","脳科学","ニューロン","AI","自由エネルギー原理","ヘッブ則","MV","ラブソング","transformer","attention"],
    },
    {
        "id": IDS["musubinaosu"],
        "key": "musubinaosu",
        "title": "【MV】結び直す朝 — 壊れても、何度でも始められる",
        "description": """朝の再出発の歌。ほどけた心も、指先で結び直せる。

▼ 歌詞
目覚ましの前に 目が覚めて
湯気の向こうで 今日が待ってる
窓の外 まだ眠る街
靴ひもをひとつ 結び直す

ささやかなことに救われて
エレの足音がリビングをめぐる
昨日の悔しさ 床に置いて
深呼吸で 畳の匂い

うまくいかない日ばかりでも
ふとした笑い声が 背中押す
大丈夫 そのままでいい

僕らは何度でも 始められる
ほどけた心も 指先で
結び直せる 結び直せる
名前を呼べば ここに戻れる

古いアルバムをひらくたび
ページの隅で時間が立ち止まる
「今日は何曜日？」 君が聞く
同じ答えでも 抱きしめたくなる

小さな会議 丸いテーブル
言葉の端っこを拾い集めて
赤いしるしで約束をつける""",
        "tags": ["朝活","再出発","MV","日本語","インディーズ","結び直す","癒し","朝","バラード"],
    },
    {
        "id": IDS["claude-code"],
        "key": "claude-code",
        "title": "【MV】Claude Code Anthem — 全部CLIでやれ🤖",
        "description": """プログラマーのアンセム。「ブラウザは閉じろ、ターミナルが戦場」

▼ 歌詞
c-l-a-u-d-e たった6文字の魔法
ブラウザは閉じろ ターミナルが戦場
「全部コマンドラインでやれ」
たったその一言で 始まるレボリューション

Python? JS? 悪くないけど
時代を裂くなら Rust か Go!
俺の推しなら 断然 Rust
コンパイル待ちでも 堅牢さで勝つ

fly deploy --remote-only
デプロイ先なら迷わず Fly.io
一度設定したら「次も覚えとけよ」
その一言で AIは覚醒

AIがビビって「2週間かかる」？
笑わせるな 無茶振りで突き刺さる！
いいからやれとエンター叩け
走らせてみりゃ 20分で終わるぜ

走らせろ 並列で! マルチエージェント!
LINEにTelegram つなげばパーフェクト!
一人ぼっちの部屋が 巨大なチーム
Claude Code 止まるな 駆け抜けろストリーム!

「全部CLIでやれ」唱え続けろ
不可能を可能にする コードを刻め!""",
        "tags": ["ClaudeCode","AI","プログラミング","Rust","CLI","Anthropic","MV","エンジニア","コーディング","Flyio","マルチエージェント"],
    },
    {
        "id": IDS["local-ai"],
        "key": "local-ai",
        "title": "【MV】Local AI — ネットなし完全無料のAIが来る",
        "description": """ローカルで動くAIの未来。クラウド不要、月額ゼロ、完全プライベート。

MLX + Qwen3.5-122B をMacで動かす方法を歌にした。

▼ 歌詞（ターミナル風）
Why LOCAL AI?
Cloud APIs — $100+/mo, code sent to servers
Ollama — slow, no Claude Code support
LM Studio — no Anthropic protocol

LOCAL AI:
MLX native — 2x faster than Ollama
Full Anthropic protocol compatibility
tool_use support — auto command execution
$0/month. Works offline.

bash setup.sh
Detecting RAM... 128GB → Tier: Ultimate
Setup complete!

~/ai.sh start
LLM 122B (Sonnet) :5000  60 tok/s
LLM 35B  (Haiku)  :5001 116 tok/s
Vision 8B         :5002
Ready!""",
        "tags": ["ローカルAI","オフラインAI","MLX","Mac","Qwen","LLM","MV","エッジAI","プログラミング","Apple Silicon"],
    },
    {
        "id": IDS["kagi"],
        "key": "kagi",
        "title": "【MV】KAGI — 深夜2時、布団から鍵を開ける🔑",
        "description": """スマートホームの歌。帰宅モード、外出モード、就寝モード、起床モード。全部スマホひとつ。

▼ 歌詞
帰宅モード — 鍵が開いて、照明がつく
外出モード — 全部消えて、鍵がかかる
就寝モード — 照明落ちて、エアコン調整
起床モード — カーテン開いて、コーヒーが淹れ始まる

深夜2時「鍵が開きません」
布団から1秒、タップ。
「開きました」
また寝る。""",
        "tags": ["スマートロック","スマートホーム","IoT","MV","KAGI","Airbnb","民泊","自動化","ホームオートメーション"],
    },
    {
        "id": IDS["koe"],
        "key": "koe",
        "title": "【MV】Koe — 声で書く、声で動かす🎙",
        "description": """音声入力アプリKoeのMV。タイピングに疲れた全ての人へ。

▼ 歌詞
朝のカフェ ラテが冷める
打ちかけのメール 言葉が出ない
会議の声 流れていく
追いかけても 指が追いつかない
タイプする毎日に 疲れてた
もっとシンプルな方法 ないのかな

キーひとつ 世界が変わる
声を聞いてくれる 相棒がいる

Koe — 声で、書く
whisperが聴いてる ぜんぶオフラインで
20の言葉を 指じゃなく 声で
WiFiなんていらない Metalが走る
あなたの声が いちばん速いキーボード

ミーティング始まる
声が文字に変わっていく リアルタイムで
終わったら AIが整えてくれる
Slackに飛ぶ Notionに残る
もう議事録で残業しない

Koe — 声で、動かす
iPhoneで話して Macに届く
「メール開いて」「スクショ撮って」
ハンズフリーの魔法

「えっと」「あの」は消えていく
メール調 チャット調 コードにだって
AIがあなたの声を磨いていく
ただ話すだけでいい あとは任せて

声で書こう Koe
声で動かそう Koe
いちばん自然な入力は
あなたの、声。""",
        "tags": ["音声入力","Koe","ボイスメモ","MV","Whisper","Mac","iPhone","オフライン","ハンズフリー","議事録","AI"],
    },
]

def make_links(exclude_key):
    lines = []
    for k, vid in IDS.items():
        if k == exclude_key:
            continue
        title = SONG_TITLES[k]
        lines.append(f"▶ {title}")
        lines.append(f"  https://youtube.com/watch?v={vid}")
    return "\n".join(lines)

def get_credentials():
    from google.auth.transport.requests import Request
    with open(TOKEN_FILE, "rb") as f:
        creds = pickle.load(f)
    if creds.expired and creds.refresh_token:
        creds.refresh(Request())
    return creds

def main():
    from googleapiclient.discovery import build

    creds = get_credentials()
    youtube = build("youtube", "v3", credentials=creds)

    for v in VIDEOS:
        links = make_links(v["key"])
        full_desc = v["description"] + "\n" + OTHER_SONGS.format(links=links)

        body = {
            "id": v["id"],
            "snippet": {
                "title": v["title"],
                "description": full_desc,
                "tags": v["tags"],
                "categoryId": "10",
                "defaultLanguage": "ja",
                "defaultAudioLanguage": "ja",
            },
        }

        try:
            youtube.videos().update(part="snippet", body=body).execute()
            print(f"OK: {v['title']}")
        except Exception as e:
            print(f"ERROR: {v['title']} — {e}")

    print(f"\n全{len(VIDEOS)}曲の説明文を更新しました!")

if __name__ == "__main__":
    main()
