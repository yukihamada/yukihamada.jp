// コードダイアグラム共有エンジン — /chords/song.html と /chords/ed.html から利用
// window.ChordDiagram = { svg(chordName), popup(chordName, subLabel) }
(function(){
  const NOTES = ['C','C#','D','Eb','E','F','F#','G','Ab','A','Bb','B'];
  const ENH = {'Db':'C#','D#':'Eb','Gb':'F#','G#':'Ab','A#':'Bb'};
  const pcOf = r => { const i = NOTES.indexOf(r); return i >= 0 ? i : NOTES.indexOf(ENH[r]); };

  // 開放系の定番シェイプ: 6弦→1弦、-1=ミュート、0=開放
  const OPEN = {
    'C':  [-1,3,2,0,1,0],  'A':  [-1,0,2,2,2,0], 'G':  [3,2,0,0,0,3],
    'E':  [0,2,2,1,0,0],   'D':  [-1,-1,0,2,3,2],'F':  [1,3,3,2,1,1],
    'B':  [-1,2,4,4,4,2],
    'Am': [-1,0,2,2,1,0],  'Em': [0,2,2,0,0,0],  'Dm': [-1,-1,0,2,3,1],
    'Fm': [1,3,3,1,1,1],   'Bm': [-1,2,4,4,3,2],
    'A7': [-1,0,2,0,2,0],  'B7': [-1,2,1,2,0,2], 'C7': [-1,3,2,3,1,0],
    'D7': [-1,-1,0,2,1,2], 'E7': [0,2,0,1,0,0],  'G7': [3,2,0,0,0,1],
    'Am7':[-1,0,2,0,1,0],  'Em7':[0,2,2,0,3,0],  'Dm7':[-1,-1,0,2,1,1],
    'Bm7':[-1,2,4,2,3,2],  'F#m':[2,4,4,2,2,2],  'C#m':[-1,4,6,6,5,4],
    'Cmaj7':[-1,3,2,0,0,0],'Fmaj7':[-1,-1,3,2,1,0],'Gmaj7':[3,2,0,0,0,2],
    'Dsus4':[-1,-1,0,2,3,3],'Asus4':[-1,0,2,2,3,0],'Esus4':[0,2,2,2,0,0],
  };

  function parse(name){
    let root = name.slice(0,2);
    if(pcOf(root) == null || pcOf(root) < 0) root = name.slice(0,1);
    if(pcOf(root) == null || pcOf(root) < 0) return null;
    return { root, qual: name.slice(root.length) };
  }

  // バレーフォールバック: Eシェイプ/Aシェイプの低い方
  function barre(root, minor){
    const pc = pcOf(root);
    const fE = ((pc - pcOf('E')) % 12 + 12) % 12 || 12; // 1..12
    const fA = ((pc - pcOf('A')) % 12 + 12) % 12 || 12;
    if(fE <= fA){
      return minor ? [fE,fE+2,fE+2,fE,fE,fE] : [fE,fE+2,fE+2,fE+1,fE,fE];
    }
    return minor ? [-1,fA,fA+2,fA+2,fA+1,fA] : [-1,fA,fA+2,fA+2,fA+2,fA];
  }

  function shapeFor(name){
    if(OPEN[name]) return OPEN[name];
    const p = parse(name);
    if(!p) return null;
    // 品質を大分類に丸める(sus/добавなどは主要形へ)
    const q = p.qual;
    const minor = /^m(?!aj)/.test(q);
    const base = p.root + (minor ? 'm' : '');
    if(q && OPEN[p.root + q]) return OPEN[p.root + q];
    if(OPEN[base]) return OPEN[base];
    return barre(p.root, minor);
  }

  function svg(name, size){
    const frets = shapeFor(name);
    if(!frets) return null;
    const played = frets.filter(f => f > 0);
    const minF = played.length ? Math.min(...played) : 1;
    const maxF = played.length ? Math.max(...played) : 4;
    const base = maxF <= 4 ? 1 : minF;           // 表示開始フレット
    const W = size || 96, H = (W * 1.25) | 0;
    const left = 14, top = 20, gw = W - left - 8, gh = H - top - 10;
    const sx = i => left + gw * i / 5;            // 弦x (6本: i=0..5)
    const fy = f => top + gh * f / 4;             // フレットy (4フレット枠)
    let out = `<svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg">`;
    out += `<style>.l{stroke:currentColor;stroke-width:1;opacity:.85}.d{fill:currentColor}.t{fill:currentColor;font:600 9px -apple-system,sans-serif}</style>`;
    // ナット or ポジション
    if(base === 1) out += `<rect x="${left}" y="${top-2.5}" width="${gw}" height="3" class="d"/>`;
    else out += `<text x="1" y="${fy(0.6)}" class="t">${base}</text>`;
    for(let i=0;i<6;i++) out += `<line x1="${sx(i)}" y1="${top}" x2="${sx(i)}" y2="${top+gh}" class="l"/>`;
    for(let f=0;f<=4;f++) out += `<line x1="${left}" y1="${fy(f)}" x2="${left+gw}" y2="${fy(f)}" class="l"/>`;
    frets.forEach((f,i)=>{
      const x = sx(i);
      if(f < 0) out += `<text x="${x-3.5}" y="${top-6}" class="t">×</text>`;
      else if(f === 0) out += `<circle cx="${x}" cy="${top-8}" r="3" fill="none" stroke="currentColor"/>`;
      else {
        const rf = f - base + 1;
        out += `<circle cx="${x}" cy="${fy(rf-0.5)}" r="5.2" class="d"/>`;
      }
    });
    out += `</svg>`;
    return out;
  }

  // 下からせり上がるポップアップ
  let pop = null;
  function popup(name, sub){
    const d = svg(name, 132);
    if(!d) return;
    if(!pop){
      pop = document.createElement('div');
      pop.id = 'chord-pop';
      pop.style.cssText = 'position:fixed;left:50%;bottom:76px;transform:translateX(-50%);z-index:60;'
        + 'background:#1a1815;border:1px solid rgba(244,241,234,.25);border-radius:14px;'
        + 'padding:12px 18px 8px;text-align:center;color:#7ecbb0;box-shadow:0 8px 30px rgba(0,0,0,.5)';
      pop.addEventListener('click', () => pop.hidden = true);
      document.body.appendChild(pop);
      document.addEventListener('click', e => {
        if(pop && !pop.hidden && !pop.contains(e.target) && !e.target.closest('[data-chord]')) pop.hidden = true;
      });
    }
    pop.innerHTML = `<div style="font:700 16px -apple-system,sans-serif;margin-bottom:2px">${name}`
      + (sub ? `<span style="font-size:11px;color:#9b968a;font-weight:400"> ${sub}</span>` : '')
      + `</div>${d}<div style="font-size:9px;color:#9b968a;margin-top:2px">タップで閉じる</div>`;
    pop.hidden = false;
  }

  window.ChordDiagram = { svg, popup, shapeFor };
})();
