---
title: "Elio Chat: Full Data From the First Week — Behind a 26.2% CVR"
date: 2026-02-16
tags: [Elio, app-launch, AI, product]
description: "Full disclosure of Elio Chat's first-week download numbers, CVR, and crash rate. A look behind the scenes of AI-native development."
---

# Elio Chat: First Week — Full Data Disclosure

On February 8, 2026, we released **Elio Chat** on the App Store.

It had been a long time since I last released an app. But **having AI made it so much easier**. All the source code and methods are up on GitHub, and I've consolidated everything under [enabler.com](https://enabler.com), which should make things a lot clearer.

One week since launch. Here are all the numbers.

---

## Key Metrics (as of February 16, 2026)

| Metric | Value | Assessment |
|--------|-------|------------|
| Total Downloads | **44** | Early stage |
| Impressions | **274** | App page view count |
| Product Page Views | **116** | Users who actually viewed the product page |
| Conversion Rate | **26.2%** | Exceptionally high (industry average: 5-10%) |
| Sessions per Device | **4.27** | Being actively used |
| Crashes | **7** | Crash rate 15.9% — needs improvement |

---

## Downloads by Region

| Country | Downloads | Share |
|---------|-----------|-------|
| Japan | 38 | 86.4% |
| United States | 2 | 4.5% |
| Australia | 1 | 2.3% |
| Denmark | 1 | 2.3% |
| United Kingdom | 1 | 2.3% |

Japan is the primary market. International reach is still in the seeding phase.

---

## Download Sources

| Source | Downloads | Share |
|--------|-----------|-------|
| App Referrer (referrals from other apps) | 20 | 45.5% |
| App Store Search (organic search) | 16 | 36.4% |
| App Store Browse | 3 | 6.8% |
| Web Referrer | 2 | 4.5% |
| Unknown | 3 | 6.8% |

**Referrals and word-of-mouth are the top channel**. That's encouraging.

---

## Downloads by Device

| Device | Downloads | Share |
|--------|-----------|-------|
| iPhone | 39 | 88.6% |
| iPad | 3 | 6.8% |
| Desktop (Mac Catalyst) | 2 | 4.5% |

The Mac Catalyst version being used was unexpected.

---

## What Went Well

### 1. A 26.2% CVR Is Exceptionally High

Against an industry average of 5-10%, **1 in 4 visitors downloaded the app**.

I believe this is evidence that the concept of a "privacy-first local AI" resonates. People who see it think, "I need this."

### 2. App Referrers at 45.5%

Without any advertising, **referrals are the top channel**. It's spreading by word of mouth.

### 3. Sessions per Device at 4.27

People aren't just downloading it and forgetting about it — **they're using it repeatedly**.

---

## Areas for Improvement

### 1. Crash Rate of 15.9%

7 crashes out of 44 downloads. **That's too high.**

The most likely cause is running out of memory during local LLM inference. This tends to happen especially on older iPhones.

**Action plan**: Strengthen memory monitoring and implement automatic model size selection based on the device.

### 2. Impressions to Product Page Views at 42.3%

274 impressions, but only 116 people actually opened the page.

**Action plan**: Optimize the App Store screenshots and icon.

### 3. Zero Reviews

No one has written a review yet.

**Action plan**: Design the right timing for in-app review prompts.

---

## How AI-Native Development Changed Things

What I really felt through this release is that **with AI, even a solo developer can ship an app**.

Before, releasing an iOS app alone was extremely tough. But now:

- **Code generation**: AI writes the boilerplate
- **Debugging**: Just paste the error message and it pinpoints the cause
- **Documentation**: AI drafts the README and privacy policy
- **Marketing**: Multi-language App Store descriptions

All the source code and methods are public on GitHub:

- **Elio GitHub**: [github.com/anthropics/elio](https://github.com/anthropics/elio)
- **Project list**: [enabler.com](https://enabler.com)

---

## Try Elio Chat!

Elio Chat is a **completely free**, **offline-capable**, **fully privacy-preserving** AI chat app.

- Runs entirely on your iPhone — zero risk of data leaks
- Works on an airplane or in the mountains
- Extensible with MCP protocol support

**Download here**:
[App Store - Elio Chat](https://apps.apple.com/app/elio-chat/id6741419362)

If you try it and have thoughts on what's great or what could be better, please let me know. Feedback is what makes the product grow.

---

## chatweb.ai Is Also Being Updated Continuously

Alongside the local AI Elio, we're continuously updating the cloud-based **chatweb.ai** as well.

### Recent Updates

- **Explore Mode**: Run multiple AI models (GPT-4o, Claude, Gemini) in parallel and pick the best answer
- **Local LLM Fallback**: Automatically switches to a local model (Qwen3-0.6B) when you can't reach the cloud
- **14 channels supported**: LINE, Telegram, Slack, Discord... chat with AI from anywhere

### When to Use Which

| | Elio Chat | chatweb.ai |
|--|-----------|------------|
| Environment | Fully local (works offline) | Cloud |
| AI Performance | Lightweight model (practical quality) | GPT-4o / Claude, etc. (highest accuracy) |
| Privacy | Maximum (zero data transmission) | Standard cloud security |
| Use Case | Handling confidential information, offline use | Tasks requiring high accuracy, multi-channel |

**Use both to the fullest!**

- Privacy first → [Elio Chat](https://apps.apple.com/app/elio-chat/id6741419362)
- High-accuracy AI → [chatweb.ai](https://chatweb.ai)

---

## Next Actions

1. **Improve crash rate** (top priority)
2. **Launch on Product Hunt**
3. **ASO optimization** (keywords and screenshot improvements)
4. **Design a review strategy**
5. **Start Android development**

---

## Related Articles

- [Offline Became the Ultimate Workspace](/blog/elio-chat-release-offline-ai-2026)
- [How to Use AI at a Company That Banned ChatGPT](/blog/offline-ai-enterprise)
- [February 2026 Monthly Report](/blog/2026-02-report)

---

**Tags**: `Elio` `App Launch` `App Store` `AI` `Product` `Data Disclosure` `chatweb.ai`
