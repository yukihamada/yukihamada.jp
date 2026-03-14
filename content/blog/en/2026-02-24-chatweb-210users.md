---
title: "210 Users and 30,000 Requests/Day — An Unfiltered Look at chatweb.ai's Business Review"
date: 2026-02-24
tags: [chatweb.ai, AI, Business, Rust, Lambda]
description: "A plan to cut monthly LLM costs from $800 to $52. An open look at the current state and strategy of chatweb.ai, an indie-developed AI service."
---

I run chatweb.ai and teai.io. Recently, I held a business review meeting with the team about the current state and future direction of the service. I believe in startup transparency, so I'm publishing the contents almost as-is.

## The Current State in Numbers

First, an honest look at where chatweb.ai stands right now.

| Metric | Value |
|--------|-------|
| Daily API requests | 30,000+ |
| Registered users | 210 |
| Monthly LLM cost | ~$800 (~120,000 yen) |
| Monthly cost per user | ~$3.8 |
| Cold start | Under 50ms |
| Supported channels | 14 (Web, LINE, Telegram, Discord, etc.) |
| Built-in tools | 35 |

The monthly cost per user is $3.8. Since the majority are on the free plan, to be honest — **we're in the red on every single user**.

## The Shocking Cost Structure

LLM costs were the most hotly debated topic in the meeting.

Currently, our primary model is MiniMax M2.5 (via OpenRouter). Input at $0.50/million tokens, output at $1.50/million tokens. Our failover targets include GPT-4o and Claude Sonnet, and when traffic routes there, costs skyrocket.

At 30,000 requests/day, that's roughly 900,000 requests per month. About $0.0009 per request on average. Not just thin margins — it's negative.

## The Savior: Nemotron Japanese Model

Enter NVIDIA's **Nemotron Nano 9B v2 Japanese**.

| Model | Input/million tok | Output/million tok |
|-------|-------------------|---------------------|
| MiniMax M2.5 (current) | $0.50 | $1.50 |
| GPT-4o (failover) | $2.50 | $10.00 |
| Claude Sonnet (last resort) | $3.00 | $15.00 |
| **Nemotron Nano 9B JP** | **$0.04** | **$0.16** |

That's **1/12.5** of the current input cost and **1/9.4** of the output cost. Projected monthly cost: $800 -> ~$52.

When you hear "9B parameters," you probably think "is the quality okay?" Honestly, it doesn't match Claude Opus or GPT-4o. But 80% of chatweb.ai's users are having everyday conversations in Japanese. Asking about the weather, getting text corrections, doing quick lookups. For those use cases, a 9B model specialized in Japanese is more than enough.

Tasks that require high quality are automatically routed to higher-tier models via our tier system. Nemotron for everyday use, Claude Sonnet when it really matters. This routing is the key to cost optimization.

## The Architecture: Why Everything Runs on a Single Lambda

Both chatweb.ai and teai.io are powered by **a single Rust binary** running on **a single AWS Lambda**.

The web UI, REST API, LINE/Telegram webhooks, OAuth, speech synthesis — everything deploys with a single `cargo build`. HTML is even embedded in the binary, so we don't use S3 or a CDN.

"Shouldn't you use microservices?" I get asked this a lot. The answer: "It's an indie project, so simplicity of operations comes first." One deploy, logs in one place, debugging in a single binary. This low operational cost is one of the reasons we can handle 30,000 requests/day on $52/month infrastructure.

## Differentiation from Competitors

The AI chat service landscape is fiercely competitive. ChatGPT, Claude, Gemini, Perplexity. Going head-to-head with the giants is a losing battle.

chatweb.ai differentiates on three points:

1. **Automatic multi-model selection**: Users don't think about models. The optimal model is selected automatically behind the scenes, with automatic failover during outages
2. **Multi-channel integration**: The same conversation continues across LINE, Telegram, Web, and Discord. LINE integration is particularly strong in the Japanese market
3. **Voice-first**: Speak via push-to-talk, get voice responses. STT + server-side TTS that works entirely in the browser

In the Japanese market especially, I see strong potential in the "AI assistant you can use on LINE" positioning.

## The Monetization Challenge

Current pricing plans:

| Plan | Monthly | Credits |
|------|---------|---------|
| Free | $0 | 100 |
| Starter | $9 | 25,000 |
| Pro | $29 | 300,000 |

To be honest, **the wall from Free to Starter is too high**. 100 credits are used up in a few conversations, but users churn before they develop the motivation to pay $9.

If Nemotron adoption brings costs down to 1/15th, we can raise the Free plan from 100 to 500 credits. This should create a natural funnel where users subscribe once they feel "I want to use this more."

## Closing Thoughts

210 users and $800/month in LLM costs. Looking at the numbers alone, the situation is tough. But we now have a clear path to running infrastructure that handles 30,000 requests/day for $52/month. A single Rust binary that cold-starts in under 50ms. Supporting 14 channels with 35 tools available.

The technical foundation is in place. What remains is how many people we can deliver this to.

I'll continue sharing openly about how far an indie-developed AI service can go.

---

*chatweb.ai is a voice-first AI assistant made in Japan. Try it for free at [chatweb.ai](https://chatweb.ai).*
*Source code is available on [GitHub](https://github.com/yukihamada/nanobot).*
