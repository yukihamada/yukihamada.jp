(function() {
  var style = document.createElement('style');
  style.textContent = [
    '#yw-btn{position:fixed;bottom:24px;right:24px;width:56px;height:56px;border-radius:50%;background:transparent;border:none;cursor:pointer;z-index:9999;display:flex;align-items:center;justify-content:center;box-shadow:0 4px 20px rgba(0,0,0,.6);transition:transform .2s;padding:0;overflow:hidden;}',
    '#yw-btn img{width:100%;height:100%;object-fit:cover;border-radius:50%;display:block;}',
    '#yw-btn:hover{transform:scale(1.08);}',
    '#yw-modal{position:fixed;bottom:88px;right:24px;width:340px;background:#111;border:1px solid rgba(255,255,255,.1);border-radius:16px;display:flex;flex-direction:column;z-index:9998;box-shadow:0 8px 40px rgba(0,0,0,.7);overflow:hidden;opacity:0;transform:translateY(16px) scale(.97);pointer-events:none;transition:opacity .22s,transform .22s;}',
    '#yw-modal.open{opacity:1;transform:translateY(0) scale(1);pointer-events:all;}',
    '#yw-head{padding:12px 16px;background:rgba(232,160,76,.08);border-bottom:1px solid rgba(255,255,255,.07);display:flex;align-items:center;gap:10px;}',
    '#yw-head img{width:32px;height:32px;border-radius:50%;object-fit:cover;}',
    '#yw-head-info{flex:1;}',
    '#yw-head-info strong{font-size:.84rem;color:#f0f0f0;display:block;font-family:inherit;}',
    '#yw-head-info span{font-size:.71rem;color:#27c93f;}',
    '#yw-reset{background:transparent;border:none;color:#555;cursor:pointer;font-size:.7rem;padding:4px 8px;border-radius:6px;font-family:inherit;}',
    '#yw-reset:hover{color:#888;background:rgba(255,255,255,.06);}',
    '#yw-msgs{overflow-y:auto;padding:14px;display:flex;flex-direction:column;gap:10px;min-height:160px;max-height:280px;}',
    '.yw-m{max-width:86%;padding:9px 13px;border-radius:12px;font-size:.82rem;line-height:1.55;white-space:pre-wrap;font-family:inherit;}',
    '.yw-m.u{align-self:flex-end;background:rgba(232,160,76,.2);color:#f0f0f0;border-bottom-right-radius:4px;}',
    '.yw-m.b{align-self:flex-start;background:rgba(255,255,255,.06);color:#d0d0d0;border-bottom-left-radius:4px;}',
    '.yw-m.t{align-self:flex-start;color:#888;font-style:italic;background:transparent;padding-left:4px;}',
    '#yw-input-row{display:flex;padding:10px 12px;gap:8px;border-top:1px solid rgba(255,255,255,.07);}',
    '#yw-input{flex:1;background:rgba(255,255,255,.07);border:1px solid rgba(255,255,255,.12);border-radius:8px;color:#f0f0f0;padding:8px 12px;font-size:.82rem;outline:none;font-family:inherit;}',
    '#yw-input:focus{border-color:#e8a04c;}',
    '#yw-send{background:#e8a04c;color:#080808;border:none;border-radius:8px;padding:8px 14px;cursor:pointer;font-size:.82rem;font-weight:700;transition:opacity .2s;font-family:inherit;}',
    '#yw-send:disabled{opacity:.35;cursor:default;}',
    '@media(max-width:480px){#yw-modal{width:calc(100vw - 32px);right:16px;bottom:80px;}#yw-btn{right:16px;bottom:16px;}}',
  ].join('');
  document.head.appendChild(style);

  // Button
  var btn = document.createElement('button');
  btn.id = 'yw-btn';
  btn.title = '濱田優貴に聞く';
  btn.innerHTML = '<img src="/assets/yuki-profile.jpg" alt="濱田優貴" onerror="this.style.display=\'none\'">';
  document.body.appendChild(btn);

  // Modal
  var modal = document.createElement('div');
  modal.id = 'yw-modal';
  modal.innerHTML =
    '<div id="yw-head">' +
      '<img src="/assets/yuki-profile.jpg" alt="Yuki" onerror="this.style.display=\'none\'">' +
      '<div id="yw-head-info"><strong>濱田優貴 AI</strong><span>● オンライン</span></div>' +
      '<button id="yw-reset" title="会話をリセット">↺ リセット</button>' +
    '</div>' +
    '<div id="yw-msgs">' +
      '<div class="yw-m b">こんにちは！Soluna、柔術、プロダクトのこと、なんでも聞いてください。</div>' +
    '</div>' +
    '<div id="yw-input-row">' +
      '<input id="yw-input" type="text" placeholder="メッセージ..." autocomplete="off" maxlength="500">' +
      '<button id="yw-send">送る</button>' +
    '</div>';
  document.body.appendChild(modal);

  // Stable anonymous user ID + HMAC token for memory (persisted in localStorage)
  var USER_ID_KEY = 'yw_user_id';
  var USER_TOKEN_KEY = 'yw_user_token';
  function getUserId() {
    var id = null;
    try { id = localStorage.getItem(USER_ID_KEY); } catch(e) {}
    if (!id) {
      var bytes = new Uint8Array(16);
      (window.crypto || window.msCrypto).getRandomValues(bytes);
      id = Array.from(bytes).map(function(b){return b.toString(16).padStart(2,'0');}).join('');
      try { localStorage.setItem(USER_ID_KEY, id); } catch(e) {}
    }
    return id;
  }
  function getUserToken() { try { return localStorage.getItem(USER_TOKEN_KEY) || null; } catch(e) { return null; } }
  function setUserToken(t) { try { localStorage.setItem(USER_TOKEN_KEY, t); } catch(e) {} }
  var userId = getUserId();

  var msgs = [];
  var open = false;
  var busy = false;

  var GREETING = 'こんにちは！Soluna、柔術、プロダクトのこと、なんでも聞いてください。';

  btn.addEventListener('click', function() {
    open = !open;
    modal.classList.toggle('open', open);
    if (open) setTimeout(function() { document.getElementById('yw-input').focus(); }, 60);
  });

  document.getElementById('yw-reset').addEventListener('click', function() {
    if (!confirm('会話履歴と記憶をすべて削除しますか？')) return;
    msgs = [];
    var box = document.getElementById('yw-msgs');
    box.innerHTML = '';
    addMsg('b', GREETING);
    // Clear server-side memory (authenticated via headers)
    var tok = getUserToken();
    if (tok) {
      fetch('/api/chat/memory', {
        method: 'DELETE',
        headers: {'X-User-ID': userId, 'X-User-Token': tok}
      }).catch(function(){});
    }
  });

  function addMsg(role, text) {
    var el = document.createElement('div');
    el.className = 'yw-m ' + role;
    el.textContent = text;
    var box = document.getElementById('yw-msgs');
    box.appendChild(el);
    box.scrollTop = box.scrollHeight;
    return el;
  }

  async function send() {
    if (busy) return;
    var input = document.getElementById('yw-input');
    var text = input.value.trim();
    if (!text) return;
    input.value = '';
    addMsg('u', text);
    msgs.push({role: 'user', content: text});
    busy = true;
    document.getElementById('yw-send').disabled = true;

    // SSE streaming response
    var botEl = addMsg('b', '');
    var fullText = '';
    try {
      var res = await fetch('/api/chat', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({messages: msgs, user_id: userId, user_token: getUserToken()}),
      });
      if (!res.ok || !res.body) throw new Error('network error');

      var reader = res.body.getReader();
      var decoder = new TextDecoder();
      var buf = '';

      while (true) {
        var result = await reader.read();
        if (result.done) break;
        buf += decoder.decode(result.value, {stream: true});
        var lines = buf.split('\n');
        buf = lines.pop() || '';
        for (var line of lines) {
          if (!line.startsWith('data: ')) continue;
          try {
            var d = JSON.parse(line.slice(6));
            if (d.user_token) { setUserToken(d.user_token); continue; }
            if (d.waiting) {
              // Show "Yuki is thinking..." indicator with animated dots
              botEl.innerHTML = '<span style="opacity:.75;font-style:italic;">' +
                d.waiting.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;') +
                '</span><span class="yw-dots">…</span>';
              if (!document.getElementById('yw-dots-style')) {
                var st = document.createElement('style'); st.id = 'yw-dots-style';
                st.textContent = '@keyframes ywdots{0%,20%{opacity:.2}50%{opacity:1}100%{opacity:.2}}.yw-dots{display:inline-block;margin-left:4px;animation:ywdots 1.2s infinite;}';
                document.head.appendChild(st);
              }
              fullText = ''; // reset for upcoming delta stream
              continue;
            }
            if (d.error) { botEl.textContent = d.error; fullText = ''; break; }
            if (d.delta) {
              if (!fullText) { botEl.textContent = ''; } // clear the "thinking" placeholder
              fullText += d.delta;
              botEl.textContent = fullText;
              var box = document.getElementById('yw-msgs');
              box.scrollTop = box.scrollHeight;
            }
          } catch(e) {}
        }
      }
      if (fullText) msgs.push({role: 'assistant', content: fullText});
      if (!fullText && !botEl.textContent) botEl.textContent = '申し訳ありません、エラーが発生しました。';
    } catch(_) {
      botEl.textContent = 'ネットワークエラーが発生しました。';
    }
    busy = false;
    document.getElementById('yw-send').disabled = false;
    input.focus();
  }

  document.getElementById('yw-send').addEventListener('click', send);
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Enter' && document.activeElement === document.getElementById('yw-input')) send();
  });
})();
