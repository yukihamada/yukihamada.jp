/*!
 * devil-reader.js — devil.pub/inner-devil 用ドロップイン (1行で導入)
 *   <script src="https://yukihamada.jp/assets/devil-reader.js" defer></script>
 *
 * 入るもの:
 *   1. アクセス解析 (enabler-analytics t.js を注入)
 *   2. 朗読モード (Web Speech API・ja-JP・現在位置から段落単位で読み上げ+ハイライト)
 *   3. メールゲート (既定: 第3章まで無料 → 以降はメール登録で解放。
 *      リードは https://yukihamada.jp/api/devil/lead に POST され Telegram 通知+JSONL 保存)
 *
 * 設定 (任意・このscriptの前に):
 *   window.DEVIL_READER = { gateAfterChapters: 3, analytics: true, tts: true, gate: true };
 */
(function () {
  "use strict";
  var CFG = Object.assign(
    { gateAfterChapters: 3, analytics: true, tts: true, gate: true,
      leadEndpoint: "https://yukihamada.jp/api/devil/lead", source: "inner-devil" },
    window.DEVIL_READER || {}
  );
  var LS_GATE = "devil_gate_email_v1";

  function ready(fn) {
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", fn);
    else fn();
  }

  /* ---------- 1. アクセス解析 ---------- */
  if (CFG.analytics && !document.querySelector('script[src*="enabler-analytics"]')) {
    var t = document.createElement("script");
    t.defer = true;
    t.src = "https://enabler-analytics.fly.dev/t.js";
    document.head.appendChild(t);
  }

  ready(function () {
    /* 本文ルート: article 優先、無ければ p が最多の main/section/div */
    function contentRoot() {
      var a = document.querySelector("article");
      if (a && a.querySelectorAll("p").length > 20) return a;
      var best = null, bestN = 0;
      ["main", "section", "div"].forEach(function (sel) {
        Array.prototype.forEach.call(document.querySelectorAll(sel), function (el) {
          var n = el.querySelectorAll(":scope > p, :scope > h2, :scope > h3").length;
          if (n > bestN) { bestN = n; best = el; }
        });
      });
      return best || document.body;
    }
    var root = contentRoot();
    var css = document.createElement("style");
    css.textContent =
      "#dvl-tts{position:fixed;bottom:18px;left:16px;z-index:90;width:52px;height:52px;border-radius:50%;border:1px solid rgba(0,0,0,.2);background:#1a1a1a;color:#fff;font-size:20px;cursor:pointer;box-shadow:0 4px 16px rgba(0,0,0,.25);}" +
      "#dvl-tts.playing{background:#8a2a1f;}" +
      ".dvl-reading{background:rgba(138,42,31,.12)!important;border-radius:4px;}" +
      ".dvl-gate-hidden{display:none!important;}" +
      "#dvl-gate{margin:48px auto;max-width:560px;padding:36px 26px;border:1.5px solid currentColor;border-radius:12px;text-align:center;font-family:-apple-system,'Hiragino Sans',sans-serif;}" +
      "#dvl-gate h3{margin:0 0 10px;font-size:18px;}#dvl-gate p{margin:0 0 18px;font-size:13.5px;opacity:.8;line-height:1.7;}" +
      "#dvl-gate form{display:flex;gap:8px;max-width:420px;margin:0 auto;}" +
      "#dvl-gate input{flex:1;padding:12px 14px;border:1px solid currentColor;border-radius:8px;font-size:16px;background:transparent;color:inherit;}" +
      "#dvl-gate button{padding:12px 20px;border:0;border-radius:8px;background:#1a1a1a;color:#fff;font-size:14px;font-weight:700;cursor:pointer;white-space:nowrap;}" +
      "#dvl-gate .msg{margin-top:10px;font-size:12.5px;opacity:.75;}" +
      "@media(max-width:480px){#dvl-gate form{flex-direction:column}}";
    document.head.appendChild(css);

    /* ---------- 3. メールゲート ---------- */
    var gated = [];
    if (CFG.gate && !localStorage.getItem(LS_GATE)) {
      var hs = Array.prototype.slice.call(root.querySelectorAll("h2"));
      if (hs.length > CFG.gateAfterChapters + 1) {
        var cut = hs[CFG.gateAfterChapters]; // この見出し以降を隠す
        var hide = false;
        Array.prototype.forEach.call(root.children, function (el) {
          if (el === cut || el.contains(cut)) hide = true;
          if (hide) { el.classList.add("dvl-gate-hidden"); gated.push(el); }
        });
        var gate = document.createElement("div");
        gate.id = "dvl-gate";
        gate.innerHTML =
          "<h3>ここから先は、メールひとつで。</h3>" +
          "<p>続き（残り " + (hs.length - CFG.gateAfterChapters) + " 章）は無料のまま読めます。<br>メールアドレスを置いていってください。次の一冊・更新の案内だけ、不定期に届きます。</p>" +
          '<form><input type="email" required placeholder="you@example.com" autocomplete="email"><button type="submit">続きを読む</button></form>' +
          '<div class="msg"></div>';
        root.appendChild(gate);
        gate.querySelector("form").addEventListener("submit", function (e) {
          e.preventDefault();
          var email = gate.querySelector("input").value.trim();
          var msg = gate.querySelector(".msg");
          msg.textContent = "送信中…";
          var progress = Math.round((window.scrollY / Math.max(1, document.documentElement.scrollHeight)) * 100) + "%";
          fetch(CFG.leadEndpoint, {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ email: email, source: CFG.source, progress: progress }),
          })
            .then(function (r) { return r.json(); })
            .then(function (j) {
              if (!j.ok) throw new Error(j.error || "error");
              localStorage.setItem(LS_GATE, email);
              gated.forEach(function (el) { el.classList.remove("dvl-gate-hidden"); });
              gate.remove();
            })
            .catch(function (err) { msg.textContent = "送信できませんでした: " + err.message; });
        });
      }
    }

    /* ---------- 2. 朗読 (Web Speech) ---------- */
    if (!CFG.tts || !("speechSynthesis" in window)) return;
    var btn = document.createElement("button");
    btn.id = "dvl-tts";
    btn.title = "ここから朗読";
    btn.textContent = "▶︎";
    document.body.appendChild(btn);

    var speaking = false, queue = [], idx = 0, current = null;
    function voiceJa() {
      var vs = speechSynthesis.getVoices().filter(function (v) { return /^ja/i.test(v.lang); });
      return vs.find(function (v) { return /Kyoko|Otoya|Google 日本語/.test(v.name); }) || vs[0] || null;
    }
    function readable() {
      return Array.prototype.filter.call(
        root.querySelectorAll("p, h2, h3, blockquote, li"),
        function (el) {
          return el.offsetParent !== null && el.textContent.trim().length > 1 && !el.closest("#dvl-gate");
        }
      );
    }
    function startFromViewport() {
      queue = readable();
      var y = window.scrollY + 90;
      idx = 0;
      for (var i = 0; i < queue.length; i++) {
        if (queue[i].getBoundingClientRect().top + window.scrollY >= y) { idx = i; break; }
      }
      next();
    }
    function clearHl() { if (current) current.classList.remove("dvl-reading"); current = null; }
    function next() {
      clearHl();
      if (!speaking || idx >= queue.length) return stop();
      current = queue[idx++];
      current.classList.add("dvl-reading");
      current.scrollIntoView({ behavior: "smooth", block: "center" });
      var text = current.textContent.replace(/\s+/g, " ").trim();
      /* 長段落は文単位に割って安定化 */
      var parts = text.match(/[^。！？]+[。！？]?/g) || [text];
      var pi = 0;
      (function speakPart() {
        if (!speaking) return;
        if (pi >= parts.length) return next();
        var u = new SpeechSynthesisUtterance(parts.slice(pi, pi + 3).join(""));
        pi += 3;
        var v = voiceJa();
        if (v) u.voice = v;
        u.lang = "ja-JP";
        u.rate = 1.02;
        u.onend = speakPart;
        u.onerror = function () { if (speaking) next(); };
        speechSynthesis.speak(u);
      })();
    }
    function stop() {
      speaking = false;
      speechSynthesis.cancel();
      clearHl();
      btn.classList.remove("playing");
      btn.textContent = "▶︎";
    }
    btn.addEventListener("click", function () {
      if (speaking) return stop();
      speaking = true;
      btn.classList.add("playing");
      btn.textContent = "■";
      if (speechSynthesis.getVoices().length === 0) {
        speechSynthesis.onvoiceschanged = function () { speechSynthesis.onvoiceschanged = null; startFromViewport(); };
        /* iOS Safari は getVoices が同期で返ることもある */
        setTimeout(function () { if (speaking && queue.length === 0) startFromViewport(); }, 300);
      } else startFromViewport();
    });
    window.addEventListener("beforeunload", stop);
  });
})();
