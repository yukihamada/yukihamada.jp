---
title: "Elio Chat Release -- Fully Offline AI, and chatweb.ai Rewritten by Its Own AI"
date: 2026-02-09
tags: [Elio, chatweb.ai, AI, iOS, Rust]
description: "Simultaneous release of Elio Chat, a fully offline AI chat app, and chatweb.ai, rewritten in Rust by an AI agent. Behind the scenes of a 37x cold-start speedup."
---

The era when "no signal" becomes the ultimate workspace has arrived. Today I am announcing two major releases at once.

## Elio Chat -- A Fully Offline AI Chat App

On a plane, on a mountaintop, in a subway tunnel. I have released **Elio Chat** for iOS -- an app that lets you talk to AI even with no connectivity.

The built-in model, **Qwen3-1.7B**, outscores GPT-3.5-turbo on the knowledge benchmark MMLU (72.5). No data is ever sent externally. Your privacy is ironclad.

An LLM running entirely inside your iPhone. What was impossible two years ago is now reality.

### Why Offline AI Matters

ChatGPT is a fantastic tool, but it has three structural limitations:

- It requires an internet connection
- Conversation data is sent to the cloud
- Companies with strict security policies prohibit its use

Elio Chat solves all three.

### Technical Highlights

| Item | Specification |
|------|------|
| Inference engine | llama.cpp (Metal GPU acceleration) |
| Supported models | 30+ |
| Smallest model size | 350MB (LFM2) |
| Maximum context | 1M tokens (Jan Nano) |
| Speech recognition | WhisperKit (fully on-device) |
| Text-to-speech | Kokoro TTS (fully on-device) |

### Five Chat Modes

Elio Chat offers five modes you can switch between depending on the situation:

1. **Local** -- Fully offline, free, maximum privacy
2. **Private** -- Connect to trusted devices on the same LAN
3. **Fast** -- Ultra-fast cloud inference via Groq API
4. **Genius** -- Top-quality AI from GPT-4o / Claude / Gemini
5. **Public** -- Connect to community servers over a P2P network

Local on a plane, Genius in the office, Fast at a cafe. Pick the best mode for every situation.

### Deep iOS Integration via MCP (Model Context Protocol)

Elio Chat adopts Anthropic's official protocol, MCP, to integrate with native iOS features:

- View, create, and delete calendar events
- Manage reminders
- Search contacts
- Get current location
- Access the photo library
- Siri Shortcuts integration

"What's on my schedule tomorrow?" "Add milk to my shopping list." -- The AI operates your iPhone's features directly.

### Strong Japanese Language Support

Elio Chat includes many models specialized for Japanese, such as **ELYZA** from the University of Tokyo's Matsuo Lab, **Swallow** from Tokyo Institute of Technology, and **TinySwallow** from Sakana AI. We also developed our own **ElioChat 1.7B v3**.

[Download on the App Store](https://apps.apple.com/jp/app/elio-chat/id6757635481)

---

## chatweb.ai -- An AI That Rewrote Itself

The other release is a complete overhaul of [chatweb.ai](https://chatweb.ai).

I told my custom AI agent, **OpenClaw**, to "rewrite chatweb.ai in Rust." OpenClaw wrote the code, ran the tests, and deployed it. I was drinking coffee.

### Performance Comparison

| Metric | Before (Node.js) | After (Rust) | Improvement |
|------|-------------------|--------------|--------|
| Cold start | 300ms | 8ms | 37x faster |
| Overall response | 2.8s | 0.3s | 9.3x faster |
| Memory usage | 150MB | 20MB | 7.5x smaller |

An 8ms cold start on Lambda. That feels essentially identical to being always on.

### The Era of AI Building AI Services

Here is a breakdown of what OpenClaw did:

1. Complete rewrite from Node.js to Rust (roughly 15,000 lines)
2. Integration with DynamoDB, Stripe, LINE, and Telegram
3. Implementation of tool calling for web search, weather, calculator, and more
4. API load balancing (round-robin + failover)
5. Writing and running tests
6. Deploying to AWS Lambda

The human contribution was one sentence -- "Rewrite it in Rust" -- plus the final review. This is the reality of 2026.

### Key Features of chatweb.ai

- **Multi-channel support** -- Talk to the same AI on Web, LINE, and Telegram
- **Web search** -- Retrieve the latest information in real time
- **Speech recognition & synthesis** -- Interact with the AI just by speaking
- **Channel sync** -- Link LINE and Web accounts via QR code
- **Privacy-focused** -- Operated in the Japan region (ap-northeast-1)

---

## Conclusion

The common theme across both products is **bringing AI back into your own hands**.

- **Elio Chat** moved AI from the cloud to the iPhone
- **chatweb.ai** handed the development of an AI service over to AI itself

Both challenge the assumption that "AI lives on the servers of giant corporations."

AI that works offline. An AI service that rewrites itself. This is the reality of 2026.
