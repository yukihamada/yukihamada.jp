# Tap, Tap, Tap — 2026.06版
## Suno カスタムモード用（コピペ2箇所: スタイル / 歌詞）

原曲 `assets/anthem.wav` のフル再生成版。2026.06追加 = 「現在地」verse（実数字）・合宿bridge・Tシャツ行・outro 2行。
生成後の宿題: whisperでword timestamps取得 → `tap.html` の `L` 配列を更新（[[kamishibai_ep4_atsume]]のwhisper検証フロー参照）。

### スタイル（Style of Music欄）
```
Japanese pop rock, upbeat, confident male vocal, talky spoken-word verses,
anthemic singalong chorus, acoustic guitar + driving beat, hand claps,
playful, hopeful, BPM 96-100, key G major
```

### 曲名
```
Tap, Tap, Tap (2026.06)
```

### 歌詞（Lyrics欄・カスタムモード）
```
[intro]
俺が使うものは、俺が作る

[verse]
2月、また来た。また胃が痛い。
でも今年は撮るだけ。パシャ。
もう終わり。

[verse]
ハンコ? 郵送? 2026年ですよ。
ポン。それだけ。終わり。

[verse]
深夜2時、「鍵が開きません」
布団から1秒、タップ。
「開きました」また寝る。

[pre-chorus]
この歌、自転車の上で書いた。
声でメモ、信号待ちで続きを足した。
アイデアって 手が空いてるときに来ない。
だから声で捕まえる。

[chorus]
Tap, tap, tap
不便だと思ったら 作ればいい
Tap, tap, tap
作ったら みんなに渡せばいい
俺もやってる、君もやろう
面倒くさいの数だけ チャンスがある
一緒にやろうぜ

[verse]
1年、絞められ続けた。
負けるパターン、全部同じだった。
フローチャートにして、アプリに入れた。
青帯。世界3位。

[verse]
あれから、6月。数字は隠さない。
柔術は161人で、月18万。
AIブランドは毎時1着、もう607案。
言葉から家が建って、1,483軒。
盛ってない。全部、晒してる。
ここまでは俺の話。ここからは、君の話。

[verse]
機内でも、無人島でも動く。
ネットなし、完全タダ、完全プライベート。
今は遅いけど、もうすぐ超賢くなる。

[verse]
10000台が同時に鳴る夜。
群衆が楽器になる。
1台は記憶、10000台はオーケストラ。

[bridge]
バグ、あります。すぐ直します。
高くしたくない。使ってほしいから。
うまくいったら 一緒に得しよう。
この夏、合宿やる。朝までコードと、馬鹿話と、焚き火。
君の席、空けてある。
気になったやつは、Tシャツ買えよ——売上も、また晒すから。

[chorus]
Tap, tap, tap
不便だと思ったら 作ればいい
Tap, tap, tap
作ったら みんなに渡せばいい
俺もやってる、君もやろう
面倒くさいの数だけ チャンスがある
一緒にやろうぜ

[outro]
パシャで確定申告、終わらせて
ポンで契約書、一秒で済ませて
KAGIで家を、スマホで動かして
Koeでアイデア、逃さないで
JiuFlowで今日も、一段強くなる
Elioに今日も、話しかけた
Solunaで今夜も、フェスは続く
ドメインは2126年まで取ってある。
2月の胃痛から、夏の焚き火まで——合宿で会おう
```

### 生成後の手順
1. Sunoで2〜4案生成 → ベスト1案をDL（wav推奨）
2. `public/mv/assets/anthem.wav` を差し替え（旧版は `anthem_2025.wav` にリネームして保持）
3. m5のfaster-whisper（[[reference_whisper_long_audio_vad]]）でword timestamps → `tap.html` の `L` 配列更新（新verseは `s:'now'`、Tシャツ行は `s:'bridge'`）
4. 英語版を作る場合は `anthem_en.wav` + `L_EN` も同様

---
*All songs generated with AI (Suno) — 濱田優貴 / Yuki Hamada*
