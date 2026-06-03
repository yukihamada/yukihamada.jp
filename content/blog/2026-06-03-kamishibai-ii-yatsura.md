---
title: "声で紙芝居をつくった — 士業 × AI × いい奴ら（と、悪魔を1匹）"
date: 2026-06-03
tags: [紙芝居, ai, 声, claude-code, エッセイ, mu]
description: "伝えたいことは、文章より2分半の紙芝居のほうが速く届く。台本だけ書いて、声も絵も字幕も全部AIに作らせた。第1話『あなたの中に、悪魔を1匹。』と、第2話『いい奴らと、世界をつくる。』── 僕の声で。"
---

長い文章を書くより、**2分半の紙芝居を1本見てもらうほうが速い**ことがある。

最近そう思って、紙芝居を作っている。台本だけ僕が書いて、**声も、絵も、字幕のタイミングも、ぜんぶAIに作らせる**。声は僕のクローン声。スマホでタップすれば、僕がしゃべりだす。

いまのところ2本ある。どっちも2分半。音が出るので、できればイヤホンで。

<div style="display:flex;flex-direction:column;gap:16px;margin:28px 0">

<a href="/kamishibai/2" style="display:block;text-decoration:none;border:1px solid #2a2a33;border-radius:16px;overflow:hidden;background:#0b0b0d">
<img src="https://devil-podcast.fly.dev/img/ep2_01_alone.webp" alt="いい奴らと、世界をつくる。" style="width:100%;display:block" loading="lazy">
<div style="padding:16px 18px;color:#f4f1ea">
<div style="color:#c0392b;font-size:12px;font-weight:700;letter-spacing:.08em">紙芝居 第2話 ・ NEW</div>
<div style="font-size:19px;font-weight:800;margin-top:4px">いい奴らと、世界をつくる。</div>
<div style="color:#8c8c98;font-size:14px;margin-top:6px">士業 × AI × いい奴ら。ひとりじゃ、世界に届かない。▶ 観る</div>
</div>
</a>

<a href="/kamishibai" style="display:block;text-decoration:none;border:1px solid #2a2a33;border-radius:16px;overflow:hidden;background:#0b0b0d">
<img src="https://devil-podcast.fly.dev/img/01_hook.png" alt="あなたの中に、悪魔を1匹。" style="width:100%;display:block" loading="lazy">
<div style="padding:16px 18px;color:#f4f1ea">
<div style="color:#c0392b;font-size:12px;font-weight:700;letter-spacing:.08em">紙芝居 第1話</div>
<div style="font-size:19px;font-weight:800;margin-top:4px">あなたの中に、悪魔を1匹。</div>
<div style="color:#8c8c98;font-size:14px;margin-top:6px">頭のいちばん上にあるものを、聴く。愛は、技術だった。▶ 観る</div>
</div>
</a>

</div>

## 第2話は「士業 × AI × いい奴ら」

会社をやってると、すぐわかる。契約も、税金も、商標も、家も、**ひとりじゃ無理**だ。専門家 ──「士業」の人たちがいる。

正直むかしは、固くて冷たい人たちだと思ってた。でもAIが来て、退屈な下調べや書類のたたきをぜんぶ引き受けてくれたら、**逆のことが起きた**。冷たい作業をAIが持ったぶん、人は「人」に集中できる。商標を出すときに会った弁理士も、税理士も、家を建ててくれる建築士も、みんなびっくりするくらい**いい奴ら**だった。

AIは速さ。士業は専門。声は距離をゼロにする。でも真ん中にいるのは、ずっと人だった。**最後の判断は、人が。** ── そういう話。

## どうやって作ったか（全部AIループ）

これ、僕が[「コードを打つ人」をやめた](/blog/2026-06-01-how-i-actually-use-claude-code)のと同じやり方で作っている。僕がやったのは台本を書くことと、最後にOKを出すことだけ。あいだは全部ループに任せた。

1. **声** — 台本を渡すと、僕のクローン声（ElevenLabs）が読み上げる
2. **絵** — 各シーンの情景を渡すと、Gem&shy;ini が同じ画風で12枚描く
3. **字幕** — Whisperで音声を文字起こしして、台本の各行を**発話のタイミングに自動で貼り付ける**（手打ちゼロ）
4. **再生** — iPhoneのサイレントスイッチでも鳴るように、WebAudioで鳴らす

絵だけで7MBあったので、最後にWebPに変換して454KB（-94%）まで落とした。表示はそのまま、速さだけ手に入れる。

## なぜ紙芝居なのか

伝えたいことの多くは、論理じゃなくて**順番と間（ま）**で決まる。文章だと読み飛ばされる「間」が、紙芝居だと効く。そして声が乗ると、急に「人」になる。

ひとりじゃ、世界に届かない。だから、いい奴らと。続きはたぶん第3話で。

▶ **[第2話を観る](/kamishibai/2)** ・ [第1話を観る](/kamishibai)
