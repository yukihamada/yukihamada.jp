#!/usr/bin/env python3
"""Audio 5-axis verification for the Transformer kamishibai.
Transcribes each transformer-kam-N.mp3 with mlx_whisper and scores vs the script.
Axes (0-100): 1.誤読なし(本文一致) 2.専門用語の読み 3.無欠落 4.尺/速度 5.総合聞き取り
Prints per-scene scores + which scenes need regeneration.
"""
import os, sys, json, re, subprocess, difflib, tempfile

HERE = os.path.dirname(__file__)
SCENES = json.load(open(os.path.join(HERE, "transformer_scenes.json"), encoding="utf-8"))
AUD = os.path.join(HERE, "..", "public", "audio")

# key terms that must survive. Each entry: list of accepted spellings (whisper may
# render English-sounding katakana as latin). Pass if ANY spelling appears.
TERMS = {
 1: [["アテンション", "attention"]],
 4: [["アテンション", "attention"]],
 6: [["マルチヘッド", "multi", "マルチ"]],
 8: [["トランスフォーマー", "トランスフォーマ", "transformer"]],
 9: [["ジーピーティー", "gpt", "ジーピー"]],
}

def norm(s):
    return re.sub(r"[、。,.\s・「」?？!！]", "", s).lower()

def dur(path):
    try:
        out = subprocess.run(["ffprobe","-v","error","-show_entries","format=duration",
                              "-of","default=nw=1:nk=1", path], capture_output=True, text=True)
        return float(out.stdout.strip())
    except Exception:
        return 0.0

def transcribe(path):
    td = tempfile.gettempdir()
    txt_path = os.path.join(td, os.path.splitext(os.path.basename(path))[0]+".txt")
    if os.path.exists(txt_path):
        os.remove(txt_path)
    subprocess.run(["mlx_whisper","--model","mlx-community/whisper-large-v3-turbo",
                    "--language","ja","--output-format","txt","--output-dir",td, path],
                   capture_output=True, text=True)
    if os.path.exists(txt_path):
        return open(txt_path, encoding="utf-8").read().strip()
    return ""

def main():
    results = []
    for i in range(1, 10):
        path = os.path.join(AUD, f"transformer-kam-{i}.mp3")
        script = SCENES[i-1]["text"]
        hyp = transcribe(path)
        ns, nh = norm(script), norm(hyp)
        sim = difflib.SequenceMatcher(None, ns, nh).ratio()
        # axis1 文字一致
        a1 = round(sim*100)
        # axis2 専門語 (each group passes if ANY accepted spelling present)
        missing = [grp[0] for grp in TERMS.get(i, []) if not any(sp.lower() in nh for sp in grp)]
        a2 = 100 if not missing else max(0, 100 - 34*len(missing))
        # axis3 無欠落 (length coverage)
        cov = min(1.0, len(nh)/max(1,len(ns)))
        a3 = round(cov*100) if cov < 1 else 100
        # axis4 尺/速度 (chars per sec, natural ~6-11)
        d = dur(path); cps = len(ns)/d if d else 0
        a4 = 100 if 3.5 <= cps <= 12.0 else (85 if 3.0 <= cps <= 13.0 else 60)
        # axis5 総合 (min of the rest)
        a5 = min(a1, a2, a3, a4)
        # a1 threshold is lenient: whisper ASR noise + English-rendered title lower exact match
        ok = a1>=88 and a2==100 and a3>=90 and a4>=85
        results.append((i, a1,a2,a3,a4,a5, missing, round(cps,1), hyp))
        flag = "OK" if ok else "REGEN"
        print(f"[{i}] 一致{a1} 用語{a2} 欠落{a3} 尺{a4}(cps{round(cps,1)}) 総合{a5}  {flag}"
              + (f"  欠語={missing}" if missing else ""))
        print(f"     heard: {hyp}")
    bad = [r[0] for r in results if not (r[1]>=97 and r[2]==100 and r[3]>=97 and r[4]>=85)]
    print("REGEN_LIST:", bad if bad else "none")
    sys.exit(0)

if __name__ == "__main__":
    main()
