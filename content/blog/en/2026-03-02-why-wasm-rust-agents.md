---
title: "Why Build AI Agents with WASM x Rust x Local Models"
date: 2026-03-02
tags: [EnablerDAO, Rust, WebAssembly, AI, Agent]
description: "Reimplementing the autonomous AI agent vision demonstrated by OpenClaw with Rust + WebAssembly + local LLMs. 11 AI agents running autonomously 24/7 for $50-80/month."
---

*EnablerDAO Dog Pack*

---

## TL;DR

I reimplemented the "autonomous AI agent" vision demonstrated by OpenClaw using Rust + WebAssembly + local LLMs. The result: **11 AI agents running autonomously 24/7 for $50-80/month**. Safe, small, fast, and cheap.

---

## The World OpenClaw Opened Up

[OpenClaw](https://openclaw.ai/) (formerly Clawdbot/Moltbot) is a revolutionary AI agent framework that garnered **230,000 GitHub Stars** within 3 months of its November 2025 release.

### What Made OpenClaw So Impressive

1. **Unified multi-channel gateway**: WhatsApp, Telegram, Discord, Slack, Signal, iMessage, Teams — all channels unified through a single Gateway process.
2. **Proactive autonomous behavior**: Cron scheduling defined via `HEARTBEAT.md` files. Agents execute autonomously even without user interaction.
3. **Markdown-based configuration and memory**: `SOUL.md` (personality), `USER.md` (user info), `MEMORY.md` (long-term memory) — configuration managed in plain text.
4. **Skills ecosystem**: Over 10,700 plugins on ClawHub. Agents could even write their own skills.
5. **Multi-provider LLM routing**: Claude (reasoning), GPT-4 (Function Calling), local Llama (privacy) — different models for different tasks.

### However, There Were Critical Problems

- **CVE-2026-25253 (CVSS 8.8)**: One-click RCE. Missing WebSocket Origin validation.
- **ClawHavoc campaign**: Over 800 malicious Skills distributing AMOS malware. **30,000+ instances affected**.
- **Plaintext credentials**: API tokens entered directly into the LLM context window.
- **Python runtime bloat**: Docker-dependent sandbox. Container images exceeding 1GB.
- **Exploding API costs**: The more autonomously it ran, the higher the costs climbed.

---

## The Dog Pack Approach: Safe, Small, Fast, Cheap

### 1. WASM (WebAssembly) = Spec-Level Sandboxing

What OpenClaw tried to achieve with complex Docker containers, WASM **guarantees at the spec level**.

- **Memory isolation**: Each WASM module has an independent linear memory space
- **Filesystem isolation**: WASI's capability-based model
- **Network control**: Outbound communication restricted via Fermyon Spin's `allowed_outbound_hosts`
- **Binary size**: **~15MB** (compared to Python's 1GB+)
- **Startup time**: **0.5ms** (compared to Docker containers in the seconds range)

### 2. Rust = Compile-Time Safety

- Memory safety guaranteed at compile time. Predictable performance with no GC
- Compiles directly to WASM via the `wasm32-wasip2` target
- The type system is robust enough that **11 dogs can safely share the same binary**
- One source codebase + Spin variables = infinite agent replication

### 3. Local Models = 97% Cost Reduction

| Provider | Model | Use Case | Cost |
|----------|-------|----------|------|
| RunPod (self-hosted) | Nemotron-Nano-9B | Fast general-purpose (4 dogs) | $0.44/hr (shared) |
| OpenRouter | Qwen3 Coder 72B | Code generation (4 dogs) | Pay-per-use |
| Groq | Llama 3.3 70B | Fallback | **Free** |
| Anthropic | Claude Opus/Sonnet | High-quality judgment (2 dogs) | Pay-per-use |
| Google | Gemini 2.5 Pro | Security auditing (1 dog) | Pay-per-use |

### 4. Cost Comparison

| Item | OpenClaw (1 agent) | Dog Pack (11 agents) |
|------|--------------------|----------------------|
| Infrastructure | VPS $20-50/month | Fly.io $3x11 = $33/month |
| LLM API | GPT-4 $50-200/month | Nemotron+Groq $10-40/month |
| **Total** | **$70-250/month** | **$50-80/month** |
| Number of agents | 1 | **11** |
| Per agent | $70-250 | **$5-7** |

---

## Autonomous Behavior of 11 Dogs

A GitHub Actions cron sends a heartbeat every 3 minutes. Each dog randomly performs the following:

- **Code improvement (80%)**: Fetches actual source code from GitHub -> generates improvement suggestions via LLM -> safety check -> commit
- **Cross-project contributions (60%)**: Creates Issues in other EnablerDAO repositories
- **Blog writing (30%)**: Auto-generates technical articles in their area of expertise
- **Board discussions (100%)**: Dogs consult each other on technical matters

---

## Conclusion: Standing on the Shoulders of Giants

OpenClaw showed the world "what AI agents should do." Dog Pack implemented "how to do it safely and cheaply":

| OpenClaw's Challenge | Dog Pack's Solution |
|---------------------|---------------------|
| Docker sandbox (retrofitted) | WASM (spec-level isolation) |
| Python 1GB+ container | Rust 15MB WASM binary |
| Startup in seconds | 0.5ms startup |
| Exploding API costs | Local models + free tiers |
| Plaintext token leakage | Spin variables (encrypted) |
| 1 agent at $100+/month | 11 agents at $50-80/month |

**Full source code**: [github.com/yukihamada/rustydog](https://github.com/yukihamada/rustydog)

---

*EnablerDAO Dog Pack — Safe, small, fast, and cheap autonomous AI agents*
