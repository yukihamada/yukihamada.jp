// 練習ツール共有エンジン — クロマチックチューナー + メトロノーム + 基準音
// /chords/ed.html 等から利用。Web Audio API のみ・外部依存なし。
// A4 = 440/442/432Hz 切替対応(432Hz派の人向け)
(function(){
  const NOTE_NAMES = ['C','C#','D','D#','E','F','F#','G','G#','A','A#','B'];
  // 標準チューニング 6弦→1弦 (A4=440基準の周波数)
  const BASE_STRINGS = [
    {name:'E2', f:82.41}, {name:'A2', f:110.00}, {name:'D3', f:146.83},
    {name:'G3', f:196.00}, {name:'B3', f:246.94}, {name:'E4', f:329.63},
  ];
  let A4 = 440;
  const ratio = () => A4 / 440;
  const STRINGS = () => BASE_STRINGS.map(s => ({ name: s.name, f: s.f * ratio() }));
  let targetBaseFreq = null; // 440基準のターゲット弦周波数

  window.setA4 = function(v){
    A4 = +v;
    document.querySelectorAll('.a4-btn').forEach(b =>
      b.classList.toggle('active', +b.dataset.a4 === A4));
    const rb = document.getElementById('refBtn');
    if(rb) rb.textContent = `♪ A=${A4}Hz 基準音`;
  };

  // ── Audio context (lazy, user gesture) ──
  let actx = null;
  function ac(){
    if(!actx) actx = new (window.AudioContext || window.webkitAudioContext)();
    if(actx.state === 'suspended') actx.resume();
    return actx;
  }

  // ── 基準音 A4 ──
  let refOsc = null, refGain = null;
  window.playRef = function(){
    const a = ac();
    const btn = document.getElementById('refBtn');
    if(refOsc){ refOsc.stop(); refOsc = null; btn.textContent = `♪ A=${A4}Hz 基準音`; return; }
    refOsc = a.createOscillator(); refGain = a.createGain();
    refOsc.frequency.value = A4; refOsc.type = 'sine';
    refGain.gain.setValueAtTime(0.0001, a.currentTime);
    refGain.gain.exponentialRampToValueAtTime(0.25, a.currentTime + 0.03);
    refOsc.connect(refGain).connect(a.destination);
    refOsc.start();
    btn.textContent = '■ 停止';
    setTimeout(()=>{ if(refOsc){ refGain.gain.exponentialRampToValueAtTime(0.0001, a.currentTime + 0.4); setTimeout(()=>{ if(refOsc){refOsc.stop(); refOsc=null; btn.textContent=`♪ A=${A4}Hz 基準音`;} }, 450); } }, 2000);
  };

  // ── チューナー ──
  let stream = null, analyser = null, raf = null, buf = null;
  const noteEl = () => document.getElementById('tunerNote');
  const centsEl = () => document.getElementById('tunerCents');
  const needleEl = () => document.getElementById('tunerNeedle');

  // オートコリレーションによるピッチ検出
  function autoCorrelate(b, sr){
    const SIZE = b.length;
    let rms = 0;
    for(let i=0;i<SIZE;i++) rms += b[i]*b[i];
    rms = Math.sqrt(rms/SIZE);
    if(rms < 0.01) return -1;
    let r1 = 0, r2 = SIZE-1;
    const th = 0.2;
    for(let i=0;i<SIZE/2;i++){ if(Math.abs(b[i])<th){ r1=i; break; } }
    for(let i=1;i<SIZE/2;i++){ if(Math.abs(b[SIZE-i])<th){ r2=SIZE-i; break; } }
    const bb = b.slice(r1, r2);
    const N = bb.length;
    if(N < 4) return -1;
    const c = new Array(N).fill(0);
    for(let i=0;i<N;i++) for(let j=0;j<N-i;j++) c[i] += bb[j]*bb[j+i];
    let d = 0;
    while(d < N-1 && c[d] > c[d+1]) d++;
    let maxval = -1, maxpos = -1;
    for(let i=d;i<N;i++){ if(c[i] > maxval){ maxval = c[i]; maxpos = i; } }
    if(maxpos <= 0) return -1;
    let T0 = maxpos;
    const x1 = c[T0-1]||0, x2 = c[T0], x3 = c[T0+1]||0;
    const aa = (x1 + x3 - 2*x2) / 2, bq = (x3 - x1) / 2;
    if(aa) T0 = T0 - bq/(2*aa);
    return sr / T0;
  }

  function freqToNote(f){
    const n = 12 * (Math.log(f / A4) / Math.LN2) + 69;
    const ni = Math.round(n);
    const cents = Math.floor((n - ni) * 100);
    return { name: NOTE_NAMES[((ni % 12) + 12) % 12] + (Math.floor(ni/12)-1), cents };
  }

  function tick(){
    if(!analyser) return;
    analyser.getFloatTimeDomainData(buf);
    let f = autoCorrelate(buf, actx.sampleRate);
    if(targetBaseFreq && f > 0){
      // ターゲット弦のオクターブ違いも拾う
      const tf = targetBaseFreq * ratio();
      if(f < tf * 0.6) f *= 2;
      if(f > tf * 1.6) f /= 2;
    }
    if(f > 0 && f < 2000){
      if(targetBaseFreq){
        const tf = targetBaseFreq * ratio();
        const cents = Math.max(-50, Math.min(50, Math.round(1200 * Math.log2(f / tf))));
        noteEl().textContent = BASE_STRINGS.find(s=>s.f===targetBaseFreq).name;
        centsEl().textContent = (cents > 0 ? '+' : '') + cents + '¢' + (Math.abs(cents) <= 5 ? ' ✓' : '');
        centsEl().style.color = Math.abs(cents) <= 5 ? 'var(--chord)' : 'var(--mut)';
        needleEl().style.left = (50 + cents) + '%';
      } else {
        const r = freqToNote(f);
        noteEl().textContent = r.name;
        centsEl().textContent = (r.cents > 0 ? '+' : '') + r.cents + '¢' + (Math.abs(r.cents) <= 5 ? ' ✓' : '');
        centsEl().style.color = Math.abs(r.cents) <= 5 ? 'var(--chord)' : 'var(--mut)';
        needleEl().style.left = (50 + Math.max(-50, Math.min(50, r.cents))) + '%';
      }
    }
    raf = requestAnimationFrame(tick);
  }

  window.toggleTuner = async function(){
    const btn = document.getElementById('tunerBtn');
    if(stream){
      stream.getTracks().forEach(t=>t.stop());
      stream = null; analyser = null;
      if(raf) cancelAnimationFrame(raf);
      btn.textContent = '🎤 チューナー開始';
      noteEl().textContent = '--';
      centsEl().textContent = '停止中';
      centsEl().style.color = 'var(--mut)';
      return;
    }
    try{
      const a = ac();
      stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation:false, noiseSuppression:false, autoGainControl:false } });
      const src = a.createMediaStreamSource(stream);
      analyser = a.createAnalyser();
      analyser.fftSize = 4096;
      buf = new Float32Array(analyser.fftSize);
      src.connect(analyser);
      btn.textContent = '■ チューナー停止';
      centsEl().textContent = '弦を鳴らしてください';
      tick();
    }catch(e){
      centsEl().textContent = 'マイクが使えません(許可を確認)';
    }
  };

  // 弦ボタン
  const strBox = document.getElementById('tunerStrings');
  if(strBox){
    BASE_STRINGS.forEach(s => {
      const b = document.createElement('button');
      b.textContent = s.name;
      b.onclick = () => {
        if(targetBaseFreq === s.f){ targetBaseFreq = null; b.classList.remove('target'); }
        else{
          targetBaseFreq = s.f;
          strBox.querySelectorAll('button').forEach(x=>x.classList.remove('target'));
          b.classList.add('target');
          // 基準音を短く鳴らす
          const a = ac();
          const o = a.createOscillator(), g = a.createGain();
          o.frequency.value = s.f * ratio(); o.type = 'triangle';
          g.gain.setValueAtTime(0.0001, a.currentTime);
          g.gain.exponentialRampToValueAtTime(0.3, a.currentTime + 0.02);
          g.gain.exponentialRampToValueAtTime(0.0001, a.currentTime + 1.4);
          o.connect(g).connect(a.destination);
          o.start(); o.stop(a.currentTime + 1.5);
        }
      };
      strBox.appendChild(b);
    });
  }

  // ── メトロノーム ──
  let metroTimer = null, bpm = 90, beat = 0;
  window.setBpm = v => {
    bpm = +v;
    document.getElementById('metroBpm').textContent = '♩=' + bpm;
    if(metroTimer){ stopMetro(); startMetro(); }
  };
  function click(accent){
    const a = ac();
    const o = a.createOscillator(), g = a.createGain();
    o.frequency.value = accent ? 1568 : 1046;
    o.type = 'sine';
    g.gain.setValueAtTime(0.0001, a.currentTime);
    g.gain.exponentialRampToValueAtTime(0.4, a.currentTime + 0.005);
    g.gain.exponentialRampToValueAtTime(0.0001, a.currentTime + 0.09);
    o.connect(g).connect(a.destination);
    o.start(); o.stop(a.currentTime + 0.1);
    const beats = document.querySelectorAll('.metro-beat');
    beats.forEach((el,i)=>el.classList.toggle('on', i === beat));
  }
  function startMetro(){
    beat = 0;
    click(true);
    metroTimer = setInterval(()=>{ beat = (beat+1)%4; click(beat===0); }, 60000/bpm);
    document.getElementById('metroBtn').textContent = '■ 停止';
  }
  function stopMetro(){
    clearInterval(metroTimer); metroTimer = null;
    document.querySelectorAll('.metro-beat').forEach(el=>el.classList.remove('on'));
    document.getElementById('metroBtn').textContent = '▶ 開始';
  }
  window.toggleMetro = () => metroTimer ? stopMetro() : startMetro();
})();
