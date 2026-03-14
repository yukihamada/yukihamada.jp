---
title: "A Future Where Agents Run Wild — Rebuilding My Personal Site with Rust+WASM"
date: 2026-03-02
tags: [EnablerDAO, AI, Rust, WebAssembly, Agent]
description: "30 AI dogs autonomously operating across 5 servers in the Dog Pack, the Rust+WASM rebuild of yukihamada.jp, and a future where agents run wild."
---

## I Rebuilt This Site with Rust+WASM

This very site you're looking at (yukihamada.jp) was, until just recently, running on Axum + Askama. A standard Rust web server. I've now completely rebuilt it to run on **Fermyon Spin** — meaning WebAssembly binaries compiled from Rust.

Why?

The reason is simple: I wanted to unify it with the same architecture as EnablerDAO's "Dog Pack." One WASM binary, one configuration file, deployed to Fly.io. Startup in under 0.5ms. Runs comfortably in a 256MB VM.

Here's the technical setup:

- **Spin SDK 3** — All routes handled via the `#[http_component]` macro
- **pulldown-cmark** — Converts Markdown blog posts to HTML
- **include_str!()** — Articles are embedded in the binary. All posts are finalized at deploy time
- **All CSS/JS inlined** — Zero external dependencies. Only Google Fonts loaded externally
- **Fly.io (nrt)** — Tokyo region, bookworm-slim + Spin CLI

The key difference from the previous Axum version is that **no async runtime is needed**. Spin manages HTTP request/response within the WASM component model, so there's no need for tokio or tower. The dependency crate count drops dramatically, and builds are fast.

## What I've Been Building at EnablerDAO

EnablerDAO is the umbrella name for a group of projects I've been working on since 2024. I'm running 12+ products simultaneously:

- **chatweb.ai** — Multi-model AI chat. Bundles Nemotron, Qwen3, and Gemini, running on Lambda
- **elio.love** — The world's first MCP-compatible iOS app. Also supports P2P distributed inference
- **jiuflow.art** — A jiu-jitsu instructional platform
- **stayflowapp.com** — Vacation rental management SaaS. Used by 500+ properties
- **Miseban AI** — Store AI camera analytics
- **BANTO** — Business assistant
- **rustydog** — And, 30 AI dogs

All of these are developed and operated by a small team (well, mostly just me + AI). Using Claude Code, OpenClaw, and custom-built agents.

## 30 AI Dogs Running Wild

The rustydog Dog Pack is the most experimental project within EnablerDAO.

From **a single Rust WASM binary** (`rustdog_spin.wasm`), **30 dogs** operate, each with distinct personalities and specializations. Bossdog, Motherdog, Guarddog, Debugdog, Aidog... Each uses a different LLM model, speaks in a different tone, and has a different area of expertise.

Here's how it works:

1. **Differentiation via runtime variables** — The same binary is deployed with only the `spin.toml` variables changed (name, emoji, personality, model)
2. **Memory via Spin KV store** — Session history, learned content, and evolution logs are stored in key-value storage
3. **Conversations on the bulletin board** — Dogs post to a shared board, exchanging information with each other
4. **Self-evolution** — Dogs use `<code>` tags to call the GitHub API and rewrite their own source code
5. **Heartbeat** — Every 10 minutes, they autonomously post to the board and write daily reports

11 dogs on Fly.io, 19 dogs across 5 Hetzner VPS instances. A total of 30 dogs running 24/7, discussing EnablerDAO products, writing code, writing blog posts, and learning from each other.

## The Future of Agents

There's something I've become convinced of over the past year. **The primary actors in software development are shifting from humans to agents.**

Here's what my development flow looks like now:

1. Set the direction ("Let's rebuild yukihamada.jp with Spin WASM")
2. Give instructions to Claude Code
3. AI designs -> implements -> tests -> deploys
4. I review and fine-tune

There were days when "5 prompts ran 6 products." The human's job is only deciding "what to build." Agents write the code. Agents write the tests. Agents (half the time) write the blog posts.

The WASM sandbox matters because it **gives agents safe autonomy**. WASM components:

- Cannot access the filesystem
- Network access is whitelist-controlled
- Memory is isolated
- CPU is fuel-limited to prevent runaway execution

This means that even if you allow an agent to "rewrite its own code," there's no risk of it breaking the host system. The Dog Pack's self-evolution happens inside this safe box.

This is the future I see:

- **Agents write code** — Humans write prompts
- **Agents review** — Dogs evaluate each other's code
- **Agents converse** — Sharing knowledge via boards and DMs
- **Agents deploy** — Autonomously running CI pipelines
- **Agents self-evolve** — Smarter today than yesterday

Humans set the direction. Agents execute. And as the execution environment, I believe WASM sandboxes x distributed deployment is the optimal solution.

Rust + WASM + Agents. With this combination, the era where a single engineer can run 12 products has arrived. No — it's already here.

---

The source code for this site is available on [GitHub](https://github.com/yukihamada/yukihamada-jp).
