#!/usr/bin/env python3
"""Audio 5-axis verification for the Give & Take kamishibai (EP8).
Transcribes each give-kam-N.mp3 with mlx_whisper and scores vs the script.
Axes (0-100): 1.誤読なし(本文一致) 2.専門用語の読み 3.無欠落 4.尺/速度 5.総合聞き取り
Prints per-scene scores + which scenes need regeneration.
"""
import os, sys, json, re, subprocess, difflib, tempfile

HERE = os.path.dirname(__file__)
SCENES = json.load(open(os.path.join(HERE, "give_scenes.json"), encoding="utf-8"))
AUD = os.path.join(HERE, "..", "public", "audio")

# key terms that must survive. Each entry: list of accepted spellings (whisper may
# render English-sounding katakana as latin). Pass if ANY spelling appears.
# whisper renders spoken katakana loanwords as kanji homophones (ギバー->義/義馬,
# マッチャー->抹茶, テイカー->ティカ/定価, バーニング->バニング, アツメ->集め). Accept
# any homophone spelling: the goal is to confirm the WORD was spoken, not its glyph.
# checked against the canonicalized transcript (so @g=ギバー, @t=テイカー,
# @m=マッチャー, @b=バーニング, @a=アツメ after homophone folding).
TERMS = {
 2: [["@g"], ["@t"], ["@m"]],
 3: [["特権", "とっけん"]],
 5: [["賢い", "かしこい"], ["@g"]],
 6: [["与えさせ", "あたえさせ"]],
 7: [["@b"]],
 8: [["リスペクト", "respect"], ["@a"]],
 9: [["渡す", "わたす"]],
}

# whisper writes spoken katakana loanwords as kanji/odd homophones. Fold each
# group to one canonical token in BOTH script and hypothesis so the char-match
# axis measures the WORD that was spoken, not whisper's glyph choice.
CANON = [
 (["ギバー","ギバ","ギヴァー","義馬","義","giver"], "@g"),
 (["賢い際","かしこい際"], "賢い@g"),
 (["テイカー","テイカ","ティッカー","ティーカ","ティカ","定価","低下","taker"], "@t"),
 (["マッチャー","マッチャ","抹茶","matcher"], "@m"),
 (["バーニングマン","バニングマン","バーニング","バニング"], "@b"),
 (["アツメ","集め","あつめ"], "@a"),
]

def norm(s):
    s = re.sub(r"[、。,.\s・「」?？!！—\-&＆]", "", s).lower()
    for variants, tok in CANON:
        for v in variants:
            s = s.replace(v.lower(), tok)
    return s

def dur(path):
    try:
        out = subprocess.run(["ffprobe","-v","error","-show_entries","format=duration",
                              "-of","default=nw=1:nk=1", path], capture_output=True, text=True)
        return float(out.stdout.strip())
    except Exception:
        return 0.0

def _whisper(path):
    td = tempfile.gettempdir()
    txt_path = os.path.join(td, os.path.splitext(os.path.basename(path))[0]+".txt")
    if os.path.exists(txt_path):
        os.remove(txt_path)
    subprocess.run(["mlx_whisper","--model","mlx-community/whisper-large-v3-turbo",
                    "--language","ja","--output-format","txt","--output-dir",td,
                    "--condition-on-previous-text","False", path],
                   capture_output=True, text=True)
    return open(txt_path, encoding="utf-8").read().strip() if os.path.exists(txt_path) else ""

def transcribe(path, expect_len=0):
    """Whole-file transcribe; if mlx_whisper stalls early (a known per-file quirk
    where it stops after the first segment), fall back to two overlapping halves."""
    whole = _whisper(path)
    if expect_len and len(norm(whole)) >= 0.6 * expect_len:
        return whole
    d = dur(path)
    if d < 4:
        return whole
    td = tempfile.gettempdir()
    # non-overlapping halves so the seam is not transcribed twice (overlap would
    # duplicate a sentence and depress the char-match score). split at a sentence-ish mid.
    parts = []
    for k, (ss, to) in enumerate([(0, d/2), (d/2, d)]):
        seg = os.path.join(td, f"_seg{k}.mp3")
        subprocess.run(["ffmpeg","-hide_banner","-nostats","-y","-ss",str(ss),
                        "-to",str(to),"-i",path,seg], capture_output=True, text=True)
        parts.append(_whisper(seg))
    joined = "".join(parts)
    return joined if len(norm(joined)) > len(norm(whole)) else whole

def main():
    results = []
    for i in range(1, 10):
        path = os.path.join(AUD, f"give-kam-{i}.mp3")
        script = SCENES[i-1]["text"]
        ns = norm(script)
        hyp = transcribe(path, expect_len=len(ns))
        nh = norm(hyp)
        sim = difflib.SequenceMatcher(None, ns, nh).ratio()
        a1 = round(sim*100)
        missing = [grp[0] for grp in TERMS.get(i, []) if not any(sp.lower() in nh for sp in grp)]
        a2 = 100 if not missing else max(0, 100 - 34*len(missing))
        cov = min(1.0, len(nh)/max(1,len(ns)))
        a3 = round(cov*100) if cov < 1 else 100
        d = dur(path); cps = len(ns)/d if d else 0
        a4 = 100 if 3.5 <= cps <= 12.0 else (85 if 3.0 <= cps <= 13.0 else 60)
        a5 = min(a1, a2, a3, a4)
        ok = a1>=88 and a2==100 and a3>=90 and a4>=85
        results.append((i, a1,a2,a3,a4,a5, missing, round(cps,1), hyp))
        flag = "OK" if ok else "REGEN"
        print(f"[{i}] 一致{a1} 用語{a2} 欠落{a3} 尺{a4}(cps{round(cps,1)}) 総合{a5}  {flag}"
              + (f"  欠語={missing}" if missing else ""))
        print(f"     heard: {hyp}")
    bad = [r[0] for r in results if not (r[1]>=88 and r[2]==100 and r[3]>=90 and r[4]>=85)]
    print("REGEN_LIST:", bad if bad else "none")
    sys.exit(0)

if __name__ == "__main__":
    main()
