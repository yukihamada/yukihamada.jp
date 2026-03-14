---
title: "Fully Offline AI and an Abandoned SaaS — A Solo Developer's Honest Business Decision"
date: 2026-02-24
tags: [Elio, 経営判断, ソロ開発, プロダクト]
description: "A product that's alive (Elio) and a product that's stalled (BANTO). A solo developer's board meeting report on the fate of two projects."
---

February 24, 2026. Monday morning. I held a board meeting with myself.

Two products were on the agenda. I had been squashing bugs on one of them just yesterday. The other hadn't seen a single code change in over a month. This meeting would decide the fate of both.

## The One That's Alive: Elio

Elio (ElioChat) is a fully offline AI chat app. It runs on iOS/macOS, supports over 30 LLM models executing entirely on-device, and requires absolutely no internet connection. Through P2P, it can directly connect devices to each other, even borrowing a friend's Mac for inference.

Yesterday alone, I fixed three bugs: a test build error, a missing QR code ID, and a missing friend-add API. Unglamorous work, but each fix was critical to the core P2P functionality. All 150 tests passing, v1.2.38 build49. One step closer to App Store review.

Elio's differentiation is crystal clear. ChatGPT, Gemini, Copilot -- none of them can do anything without the cloud. Only Elio works in airplane mode. On a plane, on a mountaintop, when communications go down in a disaster. This isn't just a technical differentiator -- it's a philosophical one. The promise that "your data never leaves your device."

## The One That Stalled: BANTO

BANTO was an invoice AI for the construction industry. The concept: "Complete an invoice using just your voice."

Technically, it was fairly robust. Over 50 database tables, 35 serverless functions, 133 tests. A solid stack of React + TypeScript + Supabase.

The problem: the backend had completely stopped working. Edge Functions were timing out, CI/CD was failing every test. The last commit was January 20. And the user count was zero.

When I stepped away a month ago, I thought, "I'll fix it next week." Next week never came. Elio bug fixes, App Store review prep, new feature development -- there was always something to do. BANTO quietly flatlined.

## Why One Kept Going and the Other Stopped

The answer is simple. **The difference in passion.**

Elio has a vision I genuinely believe in: fully offline AI. A world where AI doesn't depend on the cloud. A future where privacy is a given. Because of this vision, even tedious bug fixes don't feel like a chore. I can stay up coding until midnight to get all 150 tests to pass.

BANTO had a rational vision: "Make invoicing easier for the construction industry." The market exists. The demand is there. But I never felt that pain myself. I've never struggled with construction invoices. That's the difference between a rationally sound product and one with soul.

## The Cost of Over-Engineering

BANTO's biggest failure was building too much.

Fifty tables for a product with zero users. Thirty-five Edge Functions. One hundred thirty-three tests. This isn't proof of quality -- it's evidence of over-engineering. I tried to build the perfect architecture before finding the first ten users. I was validating the integrity of tables nobody used, with tests nobody read.

By contrast, Elio follows a cycle of "build something that works first, then fix it as you use it." Just yesterday, I tried adding a friend via P2P, found a bug, and fixed it the same day. That's the difference between development driven by user experience and development that starts with table design.

## The Decision to Freeze

Today, I froze BANTO.

Not killed -- frozen. The codebase will be preserved. The 50 tables, 133 tests -- those are assets that might prove useful for something in the future. But right now, there's no reason to allocate limited resources to BANTO.

Instead, I'm pouring all my energy into Elio. Getting through App Store review. Reaching the first users. Getting as many people as possible to experience the heretical vision of offline AI.

## To Fellow Solo Developers

If you're in the same situation, there's one thing I want to say.

**Have the courage to stop.**

Holding on to a stalled product, telling yourself "I'll fix it someday," is stealing resources from the product that's actually alive. Freezing isn't failure. It's a strategic decision to focus.

I don't know how Elio's App Store review will turn out. But at least today, right now, I've narrowed my focus to a single product. That alone made today's board meeting worthwhile.

---

*Elio (ElioChat): [elio.love](https://elio.love)*
*BANTO: Frozen*
