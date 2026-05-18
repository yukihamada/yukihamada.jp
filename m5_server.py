#!/usr/bin/env python3
"""
m5 HITL server — yukihamada.jp / chat + transcription
Runs on Mac (192.168.0.47), exposed via Cloudflare tunnel.

Setup:
  pip install fastapi uvicorn faster-whisper httpx
  python3 m5_server.py
  # in another terminal:
  cloudflared tunnel --url http://localhost:8888
  # copy the https://xxx.trycloudflare.com URL, then register:
  curl -X POST https://yukihamada.jp/api/m5/register \
    -H "Content-Type: application/json" \
    -d '{"url":"https://xxx.trycloudflare.com","token":"<M5_REGISTER_TOKEN>"}'
"""
import os, io, tempfile, time, json, asyncio, threading
from typing import Optional

from fastapi import FastAPI, HTTPException, File, UploadFile, Form, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel
import httpx
import uvicorn

app = FastAPI()
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

ANTHROPIC_API_KEY = os.environ.get("ANTHROPIC_API_KEY", "")
PORT = int(os.environ.get("M5_PORT", "8888"))

# ── State tracking ──
_state = {"status": "idle", "last_query": "", "last_updated": 0}
_state_lock = threading.Lock()

def set_state(status: str, query: str = ""):
    with _state_lock:
        _state["status"] = status
        _state["last_query"] = query
        _state["last_updated"] = time.time()

def get_state():
    with _state_lock:
        return dict(_state)

# ── Whisper (lazy-loaded) ──
_whisper_model = None
_whisper_lock  = threading.Lock()

def get_whisper(model_size: str = "large-v3"):
    global _whisper_model
    with _whisper_lock:
        if _whisper_model is None:
            print(f"Loading faster-whisper {model_size}...", flush=True)
            try:
                from faster_whisper import WhisperModel
                _whisper_model = WhisperModel(model_size, device="cpu", compute_type="int8")
                print("faster-whisper ready!", flush=True)
            except ImportError:
                print("faster-whisper not found, trying openai-whisper...", flush=True)
                import whisper as _w
                _whisper_model = _w.load_model("turbo")
                print("whisper ready!", flush=True)
    return _whisper_model

# ── Routes ──

@app.get("/")
def root():
    return {"ok": True, "service": "yukihamada-m5-hitl"}

@app.get("/health")
def health():
    return {"ok": True}

@app.get("/state")
def state():
    return {"current": get_state()}

class AskReq(BaseModel):
    question: str
    context: Optional[str] = ""
    site: Optional[str] = "yuki"
    user_id: Optional[str] = None

@app.post("/ask")
async def ask(req: AskReq):
    """Chat endpoint — called by yukihamada.jp Rust server."""
    set_state("thinking", req.question)
    try:
        answer = await _ask_claude(req.question, req.context or "")
        set_state("idle")
        return {"text": answer, "ok": True}
    except Exception as e:
        set_state("idle")
        raise HTTPException(status_code=500, detail=str(e))

async def _ask_claude(question: str, context: str) -> str:
    set_state("waiting_slow")
    system = (
        "あなたは濱田優貴（Yuki Hamada）のパーソナルサイト yukihamada.jp のAIアシスタントです。\n"
        "訪問者の質問に、濱田優貴の言葉として自然な日本語で簡潔に答えてください。\n"
        "英語で質問されたら英語で答えてください。\n"
        "マークダウン記法は使わず、普通のテキストで答えてください。\n\n"
        "# 濱田優貴について\n"
        "- Enabler（イネブラ）代表取締役CEO\n"
        "- 元メルカリ 取締役 CPO/CINO（2014〜2021）\n"
        "- 元NOT A HOTEL 共同創業者（2018〜2024）\n"
        "- 柔術青帯\n"
        "- 「建てて、残して、いいやつらと。」がモットー\n"
        "- Rust・Swift・ESP32を自分で書く実装型CEO\n"
    )
    if context:
        system += f"\n\n{context}"

    async with httpx.AsyncClient(timeout=120.0) as client:
        set_state("editing")
        # Use local Qwen3 via mlx_lm OpenAI-compatible server (port 5010)
        resp = await client.post(
            "http://localhost:5010/v1/chat/completions",
            json={
                "model": "mlx-community/Qwen3.5-9B-4bit",
                "max_tokens": 800,
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user", "content": question},
                ],
            },
        )
        resp.raise_for_status()
        data = resp.json()
        return data["choices"][0]["message"]["content"].strip()

@app.post("/transcribe")
async def transcribe(audio: UploadFile = File(...)):
    """
    Transcribe audio using local Whisper.
    Accepts multipart/form-data with 'audio' field.
    Returns {"text": "...", "language": "ja", "source": "m5-whisper"}
    """
    audio_bytes = await audio.read()
    if len(audio_bytes) < 100:
        raise HTTPException(status_code=400, detail="audio too short")
    if len(audio_bytes) > 50_000_000:
        raise HTTPException(status_code=413, detail="audio too large (max 50MB)")

    filename = audio.filename or "audio.webm"
    suffix = "." + filename.rsplit(".", 1)[-1] if "." in filename else ".webm"

    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as tmp:
        tmp.write(audio_bytes)
        tmp_path = tmp.name

    try:
        model = get_whisper()
        text = await asyncio.get_event_loop().run_in_executor(
            None, _transcribe_sync, model, tmp_path
        )
        return {"text": text, "language": "ja", "source": "m5-whisper"}
    finally:
        try: os.unlink(tmp_path)
        except: pass

def _transcribe_sync(model, path: str) -> str:
    """Run whisper synchronously in thread pool."""
    try:
        # faster-whisper API
        segments, info = model.transcribe(
            path,
            language="ja",
            beam_size=5,
            vad_filter=True,
            vad_parameters={"min_silence_duration_ms": 500},
        )
        return "".join(seg.text for seg in segments).strip()
    except TypeError:
        # openai-whisper API (different signature)
        result = model.transcribe(path, language="ja", fp16=False)
        return result["text"].strip()


if __name__ == "__main__":
    print(f"Starting m5 HITL server on port {PORT}", flush=True)
    print("Endpoints: /ask  /transcribe  /state  /health", flush=True)
    if not ANTHROPIC_API_KEY:
        print("⚠ ANTHROPIC_API_KEY not set — /ask will return placeholder", flush=True)
    print("", flush=True)
    print("To expose via Cloudflare tunnel:", flush=True)
    print(f"  cloudflared tunnel --url http://localhost:{PORT}", flush=True)
    uvicorn.run(app, host="0.0.0.0", port=PORT, log_level="info")
