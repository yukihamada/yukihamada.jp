#!/usr/bin/env python3
"""kamishibai v2 テンプレ生成 — v2脚本JSONからシーン型プレイヤーHTMLを生成

- EP1-EP6/EP8: EP4型スケルトンから生成(EP1/EP2は旧キャプション同期型を置換=外部オリジン依存も解消)
- EP7: 既存テンプレのSCENESのみ差し替え(SVG図解・MVは維持)
- magmag: 対象外(private overlay)
"""
import json, re, os

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
V2 = lambda ep: json.load(open(f"{BASE}/scripts/kamishibai_v2/{ep}.json"))

PREFIX = {"EP1":"ep1v2","EP2":"ep2v2","EP3":"asoview","EP4":"atsume","EP5":"kagi","EP6":"minna","EP7":"transformer","EP8":"give","BLANK":"blank"}
FILE = {"EP1":"kamishibai.html","EP2":"kamishibai-ep2.html","EP3":"kamishibai-ep3.html","EP4":"kamishibai-ep4.html",
        "EP5":"kamishibai-ep5.html","EP6":"kamishibai-ep6.html","EP7":"kamishibai-ep7.html","EP8":"kamishibai-ep8.html",
        "BLANK":"kamishibai-blank.html"}

META = {
 "EP1": dict(no=1, sub="トップオブマインド編", robots="noindex,nofollow", og=11,
   desc="頭のいちばん上に、いつも座っている誰か。愛は、感情じゃなく技術。AIジャッジ499/500の改稿版・全11場面。"),
 "EP2": dict(no=2, sub="いい奴ら編", robots="index,follow", og=7,
   desc="ひとりで世界に届くと思ってた。AIに冷たい仕事を渡した日、専門家が人に見えた。改稿版・全8場面。"),
 "EP3": dict(no=3, sub="任せ方編", robots="index,follow", og=6,
   desc="任せると雑になる。だからぜんぶ自分でやった——いちばん雑だったのは、ぼくでした。改稿版・全7場面。"),
 "EP4": dict(no=4, sub="ATSUME 焚き火編", robots="index,follow", og=5,
   desc="星、ふたつ。AIが私の全部につけた点数。一人で世界には届かない——だから火を分け合う。改稿版・全8場面。"),
 "EP5": dict(no=5, sub="AI時代のセキュリティ編", robots="index,follow", og=8,
   desc="作る速さが100倍なら、鍵のかけ忘れも100倍。公開する前に、一度、泥棒になってください。改稿版・全11場面。"),
 "EP6": dict(no=6, sub="協力の証明編", robots="index,follow", og=10,
   desc="情緒をぜんぶ抜いて、計算だけで「一人でやる」を証明しようとしたら、正反対が証明された。改稿版・全11場面。"),
 "EP8": dict(no=8, sub="ギバー編", robots="index,follow", og=9,
   desc="いちばん損するのも、いちばん遠くへ行くのも、先に渡す人。違いはひとつ——自分も守れるか。改稿版・全9場面。"),
 "BLANK": dict(no=0, label="番外編", sub="BLANK 001 弟子屈編", robots="index,follow", og=4,
   desc="白帯と、空のプロンプトは同じ色。四人で弟子屈へ——3日で一本取って、一本作る。全9場面。"),
}

TITLES = {
 "EP1": {1:("問い","だれが、いますか。"),2:("こころの中","いない、と思った。"),3:("こいつ","小さな悪魔。"),4:("悪魔の声","「お前は変われない」"),5:("問い、ふたたび","人は、どう変わる?"),6:("裏返る","気合いじゃ、なかった。"),7:("いちばん上","世界の色を決める声。"),8:("置いておくと","優しさも、皮肉に。"),9:("芯","愛は、技術。"),10:("まず、こいつから","責めなくなった。"),11:("むすび","あなたの、いちばん上。")},
 "EP2": {1:("夜中の二時","ハンコを、押せない。"),2:("ひとりで","誰の手も、いらない?"),3:("最初のひとり","言葉を、守る人。"),4:("ふたり目","専門家って、人だ。"),5:("裏返る","逆だった。"),6:("芯","真ん中にいるのは、人。"),7:("ふたたび、夜中の二時","満席だった。"),8:("あなたへ","隣に、いい奴らは?")},
 "EP3": {1:("白状","いちばん雑だったのは。"),2:("火のそば","「なんで自分でやってるんですか」"),3:("任せ方、四つ","手が、空いてる。"),4:("裏返る","退屈はAIに。ワクワクは人に。"),5:("空気のように","ここからが、人の番だ。"),6:("芯","最後の魔法は、人がかける。"),7:("あなたへ","また、火の前で。")},
 "EP4": {1:("査定","星、ふたつ。"),2:("その夜","点数は、正しかった。"),3:("火をおこす","一本じゃ、燃えない。"),4:("裏返る","分け合って、燃え上がる。"),5:("芯","一人で、世界には届かない。"),6:("空席","やさしい沈黙は、高くつく。"),7:("だから","くべるまで、ただの薪。"),8:("あなたへ","隣の席は、誰のために。")},
 "EP5": {1:("玄関","かけた、はず。"),2:("速さ百倍","すごい時代です。"),3:("裏返る","かけ忘れも、百倍。"),4:("白状","七つのうち、三つ。"),5:("芯","できたつもりが、いちばん危ない。"),6:("その一","迷ったら、閉じる。"),7:("その二","合鍵を、マットの下に置かない。"),8:("その三","一度、泥棒になる。"),9:("見える","開いてる窓が、見つかる。"),10:("思想","安全は、速さの一部。"),11:("むすび","かけて、確かめてから。")},
 "EP6": {1:("宣言","一人でやる人間です。"),2:("前提","計算だけで、考える。"),3:("証明 一","十倍、早い。"),4:("証明 二","盲点は、掛け算で消える。"),5:("証明 三","当たりは、人の数だけ。"),6:("証明 四","計算が、おかしい。"),7:("証明 五","下手な人がいて、最大になれる。"),8:("証明 六","わけても、減らない。"),9:("反転","正反対でした。"),10:("むすび","みんなの火は、朝まで。"),11:("あなたへ","火のそばで、会いましょう。")},
 "EP7": {1:("二〇一七年","注意こそ、すべて。"),2:("むかし","順番待ちで、遅い。"),3:("ひらめき","机の上に、ぜんぶ広げる。"),4:("アテンション","どこを見るか、自分で決める。"),5:("しくみ","似ているものに、惹かれる。"),6:("複数の目","一人で、見ない。"),7:("順番の情報","波の印を、そっと足す。"),8:("結果","順番待ちが、消えた。"),9:("それから","あの題名は、ほんとうだった。"),10:("きみへ","さいごに、一曲。")},
 "EP8": {1:("常識","親切な人は、損をする?"),2:("三人","ギバー、テイカー、マッチャー。"),3:("意外","いちばん上も、与える人。"),4:("裏返る","自分も、守るかどうか。"),5:("橋","向ける先を、選ぶ。"),6:("作法","賢く、配る。"),7:("砂漠の街","先に渡す、それだけ。"),8:("むすび","すり減らさずに。"),9:("あなたへ","かしこく、先に、渡す。")},
 "BLANK": {1:("白","同じ色を、してる。"),2:("誘い","「弟子屈、行かない？」"),3:("四人","手が、挙がった。"),4:("北へ","街が、湖に変わる。"),5:("朝","体の、一本。"),6:("夜","頭の、一本。"),7:("白だから","まだ、誰も知らない。"),8:("芯","動詞は、「組む」。"),9:("あなたへ","残りの席は、あなたの分。")},
}

CTA = {  # ep -> {scene_n: (href, label)}  控えめに最終盤1箇所のみ
 "EP2": {8:("https://atsm.wtf","🔥 火を囲みに")},
 "EP4": {8:("https://atsm.wtf/community","🔥 あなたの薪を、火に")},
 "EP5": {11:("/security-gate","🔑 その七つの質問は、ここに")},
 "EP6": {11:("https://atsm.wtf","🔥 焚き火へ")},
 "EP8": {9:("https://atsm.wtf","🔥 最初の一本を")},
 "BLANK": {9:("/blank","⬜ 残りの席を見る — BLANK 001")},
}

CTA2 = {  # ep -> {scene_n: (href, label)}  2本目のリンク(最終場面のみ・控えめ)
 "BLANK": {9:("https://wearmu.com/shop/BLANKCAMP-AGENT-TEE-WHITE-375b9cd6","👕 席が遠い人は、白を着る — BLANK 001 Tee ¥4,900")},
}

SKELETON = open(f"{BASE}/templates/kamishibai-ep4.html").read()

def scenes_json(ep):
    d = V2(ep); pre = PREFIX[ep]; out = []
    for sc in d["scenes"]:
        n = sc["n"]; sub, title = TITLES[ep][n]
        href, label = (CTA.get(ep) or {}).get(n, ("", ""))
        href2, label2 = (CTA2.get(ep) or {}).get(n, ("", ""))
        out.append({"sub": sub, "title": title, "text": sc["narration"],
                    "img": f"/assets/kamishibai-v2/{ep}/scene{n}.png",
                    "audio": f"/audio/v2/{pre}-kam-{n}.mp3", "buy": href, "cta": label,
                    "buy2": href2, "cta2": label2})
    return out

def build_standard(ep):
    d = V2(ep); m = META[ep]; sc = scenes_json(ep)
    html = SKELETON
    # head 差し替え
    epno = m.get("label", f"第{m['no']}話")
    html = re.sub(r"<title>.*?</title>",
        f"<title>紙芝居 {epno}『{d['title']}』｜{m['sub']} — 濱田優貴</title>", html, flags=re.S)
    html = re.sub(r'<meta name="description" content=".*?">',
        f'<meta name="description" content="{m["desc"]}">', html)
    html = re.sub(r'<meta property="og:title" content=".*?">',
        f'<meta property="og:title" content="紙芝居 {epno}『{d["title"]}』">', html)
    html = re.sub(r'<meta property="og:description" content=".*?">',
        f'<meta property="og:description" content="{m["desc"]}">', html)
    html = re.sub(r'<meta property="og:image" content=".*?">',
        f'<meta property="og:image" content="https://yukihamada.jp/assets/kamishibai-v2/{ep}/scene{m["og"]}.png">', html)
    html = html.replace("</title>", f'</title>\n<meta name="robots" content="{m["robots"]}" />', 1)
    # タイトル画面
    html = re.sub(r"<h1>.*?</h1>", f"<h1>{d['title']}</h1>", html)
    note = "2026.06 改稿版" if "label" not in m else "2026.06"
    html = re.sub(r"<p>紙芝居.*?</p>",
        f"<p>紙芝居 {epno} ・ {m['sub']} ・ {note}<br>声：濱田優貴（AIクローン）</p>", html, flags=re.S)
    # SCENES
    html = re.sub(r"const SCENES=\[.*?\];",
        "const SCENES=" + json.dumps(sc, ensure_ascii=False) + ";", html, flags=re.S)
    # 2本目のCTA (CTA2登録episodeのみ): 要素+CSS+JSを注入
    if CTA2.get(ep):
        html = html.replace('<a id="scene-link" target="_blank" rel="noopener"></a>',
            '<a id="scene-link" target="_blank" rel="noopener"></a>\n    <a id="scene-link2" target="_blank" rel="noopener"></a>')
        html = html.replace('#scene-link.show{display:inline-block}',
            '#scene-link.show{display:inline-block}\n  #scene-link2{display:none;margin-top:10px;margin-left:12px;padding:12px 32px;border:1px solid var(--ink,#888);color:inherit;opacity:.85;letter-spacing:.16em;font-size:15px;border-radius:3px;text-decoration:none;font-family:"Hiragino Kaku Gothic ProN",sans-serif}\n  #scene-link2.show{display:inline-block}')
        html = html.replace("else link.classList.remove('show');",
            "else link.classList.remove('show');\n  const link2=document.getElementById('scene-link2');\n  if(s.buy2){ link2.href=s.buy2; link2.textContent=s.cta2; link2.classList.add('show'); }\n  else link2.classList.remove('show');")
        html = html.replace("document.getElementById('scene-link').addEventListener('click',e=>e.stopPropagation());",
            "document.getElementById('scene-link').addEventListener('click',e=>e.stopPropagation());\ndocument.getElementById('scene-link2').addEventListener('click',e=>e.stopPropagation());")
    open(f"{BASE}/templates/{FILE[ep]}", "w").write(html)
    print(f"{FILE[ep]}: {len(sc)} scenes" + (" +CTA2" if CTA2.get(ep) else ""))

def build_ep7():
    d = V2("EP7"); sc = scenes_json("EP7")
    DGM = {2:"seq",3:"spread",4:"attn",5:"qkv",6:"multi",7:"pos",8:"result"}
    for i, s in enumerate(sc):
        n = i + 1
        if n in DGM: s["dgm"] = DGM[n]
        if n == 10: s["mv"] = True
    html = open(f"{BASE}/templates/kamishibai-ep7.html").read()
    html = re.sub(r"const SCENES=\[.*?\];",
        "const SCENES=" + json.dumps(sc, ensure_ascii=False) + ";", html, flags=re.S)
    html = re.sub(r"<p>紙芝居.*?</p>",
        "<p>紙芝居 第7話 ・ Transformer編 ・ 2026.06 改稿版<br>声：濱田優貴（AIクローン）</p>", html, flags=re.S)
    open(f"{BASE}/templates/kamishibai-ep7.html", "w").write(html)
    print(f"kamishibai-ep7.html: {len(sc)} scenes (DIAG/MV維持)")

if __name__ == "__main__":
    import sys
    targets = sys.argv[1:] or ["EP1","EP2","EP3","EP4","EP5","EP6","EP8","EP7"]
    for ep in targets:
        build_ep7() if ep == "EP7" else build_standard(ep)
    print("done")
