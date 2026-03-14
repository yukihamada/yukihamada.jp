---
title: "February 2026 Monthly Report - The Reality (and Failures) of Running 8 Services Simultaneously"
date: 2026-02-14
tags: [Monthly Report, Startup, SaaS, AI]
description: "The reality of running 8 services solo. Visitor counts, MRR, failures, and lessons -- all laid bare."
---

# February 2026 Monthly Report

**Operator**: Yuki Hamada ([@yukihamada](https://twitter.com/yukihamada))
**Company**: Enabler Inc.
**Services in operation**: 8
**Report period**: February 1-14, 2026 (mid-month update)

---

## Summary

### Overall Numbers (February 1-14)

| Metric | Value |
|--------|-------|
| Total visitors | 18,200 |
| Total MRR | 2.78 million yen |
| Total development hours | 112 hours |
| Coffee consumed | 42 cups |
| Average sleep | 5.2 hours/day |

---

## Service-by-Service Report

### 1. chatweb.ai (AI Agent Platform)

**Overview**: Multi-model AI agent with 14-channel support

**Numbers**:
- Visitors: 5,200 (+12% MoM)
- Registered users: 1,850 (+8% MoM)
- Active users: 820
- MRR: 480,000 yen (+5% MoM)

**Highlights**:
- **New feature**: Launched Explore Mode (parallel execution across multiple AI models)
- **Technical**: Implemented Local LLM Fallback (Qwen3-0.6B)
- **Marketing**: Product Hunt post -- 320 upvotes

**Challenges**:
- Credits are consumed too quickly (Free Plan 100 credits -- many users burn through them in a day)
- Channel-specific prompt optimization needed

**Lessons**:
- Explore Mode is used more than expected (DAU of 120)
- Inflow from LINE Bot is steady (400 users/month)

---

### 2. jitsuflow.app (BJJ Dojo Management SaaS)

**Overview**: Management app for Brazilian Jiu-Jitsu dojos

**Numbers**:
- Visitors: 1,200 (-3% MoM)
- Registered dojos: 85
- Members: 1,300
- MRR: 1.02 million yen (+1% MoM)

**Highlights**:
- **New feature**: Video library (Cloudflare Stream integration)
- **Churn**: 2 dojos cancelled (reason: cost reduction)
- **New contracts**: 3 dojos (2 of which are international)

**Challenges**:
- Churn rate slightly increasing (2.3% to 2.8%)
- Support workload up to 7 hours/week

**Lessons**:
- Video feature is popular, but upload size limits are causing complaints
- Sensing potential for international expansion (need to strengthen English support)

---

### 3. yukihamada.jp (Portfolio Site)

**Overview**: Personal blog and portfolio

**Numbers**:
- Visitors: 3,800 (+18% MoM)
- Pageviews: 8,200
- Average session duration: 2 minutes 30 seconds
- Bounce rate: 42%

**Highlights**:
- **New posts**: 3 published (AI, BJJ, domain investing)
- **Viral hit**: "The Time I Accidentally Racked Up 8.45 Million Yen" got 23,000 likes on X
- **SEO**: Ranked #3 for "BJJ dojo management"

**Challenges**:
- Newsletter signups not growing (12 per month)
- Social media posting not yet automated

**Lessons**:
- Viral content loves failure stories
- SEO is a long game -- just keep at it

---

### 4. news.online / news.xyz (Short-Form News Platform)

**Overview**: AI-summarized news in 240 characters or less

**Numbers**:
- Visitors: 4,200 (+25% MoM)
- Articles: 180 (monthly)
- Twitter followers: 1,850 (+15% MoM)
- MRR: 170,000 yen (sponsored articles + paid newsletter)

**Highlights**:
- **Sponsored articles**: 5 booked (30,000 yen x 5 = 150,000 yen)
- **Paid newsletter**: 22 subscribers (980 yen/month x 22 = 21,560 yen)
- **Technical**: Introduced Jina Reader, improving scraping accuracy

**Challenges**:
- Article creation is too manual (need automation)
- Twitter's algorithm remains a mystery

**Lessons**:
- There is demand for sponsored articles
- Short-form content can still deliver value

---

### 5. Elio (Local AI Agent for iOS)

**Overview**: Privacy-focused offline AI

**Numbers**:
- TestFlight users: 180
- Active users: 42
- GitHub Stars: 320 (+45 MoM)
- MRR: 0 yen (not yet monetized)

**Highlights**:
- **New feature**: Enhanced MCP Server integration
- **Press**: Made the Hacker News Top 10
- **Enterprise inquiries**: 3 companies (exploring enterprise deployment)

**Challenges**:
- Monetization strategy unclear
- Frequent requests from Android users: "When is it coming?"

**Lessons**:
- There is definite demand for privacy-focused solutions
- Open source projects can still generate revenue (support contracts, etc.)

---

### 6. webllm.app (LLM in the Browser)

**Overview**: LLM running on WebAssembly + WebGPU

**Numbers**:
- Visitors: 2,100 (+8% MoM)
- Unique users: 1,400
- MRR: 0 yen

**Highlights**:
- **Technical**: Added Qwen3-0.6B-GGUF support
- **Viral**: "AI running in just a browser" tweet got 12,000 likes

**Challenges**:
- Zero monetization
- Server costs (currently running on Cloudflare's free Edge tier)

**Lessons**:
- Fun as a demo, but monetization is difficult
- Effective as a tech showcase

---

### 7. voiceGPTweb (Voice AI)

**Overview**: Talk to ChatGPT with your voice

**Numbers**:
- Visitors: 800 (-12% MoM)
- MRR: 0 yen

**Highlights**:
- **Status**: Essentially in maintenance mode
- **Technical**: Still running on legacy PHP code

**Challenges**:
- No updates being made
- Completely outpaced by competitors (ChatGPT Voice Mode)

**Lessons**:
- Should outdated products be gracefully retired?
- But there's still some traffic (mysteriously)

---

### 8. Other (Experimental Projects)

**Small-scale projects on GitHub**:

- **groq-cli**: Groq API CLI (150 visitors/month)
- **flyagi**: AI agent on Fly.io (80 visitors/month)
- **godseye**: Monitoring tool (50 visitors/month)

**MRR**: 0 yen
**Development time**: Less than 5 hours/month

**Lesson**: Small projects can be left on autopilot. Even just accumulating GitHub Stars has value.

---

## Monthly Financial Summary

### Revenue (MRR)

| Service | MRR |
|---------|-----|
| chatweb.ai | 480,000 yen |
| jitsuflow | 1,020,000 yen |
| news.online/xyz | 170,000 yen |
| yukihamada.jp | 0 yen |
| Elio | 0 yen |
| Other | 0 yen |
| **Total** | **1,670,000 yen** |

### Costs

| Item | Amount |
|------|--------|
| AWS Lambda (chatweb.ai) | 120,000 yen |
| Firebase (jitsuflow) | 32,000 yen |
| Supabase (multiple services) | 45,000 yen |
| Cloudflare (all services) | 20,000 yen |
| Stripe fees | 60,000 yen |
| Domain costs | 8,000 yen |
| Other SaaS | 25,000 yen |
| **Total** | **310,000 yen** |

### Profit

**1,670,000 yen (MRR) - 310,000 yen (costs) = 1,360,000 yen**

(Note: Development time costs not included.)

---

## Failure Stories

### Failure 1: Neglecting SEO for news.xyz

On February 1, I checked Google Search Console and found only **12 pages indexed**.

Even though 180 articles had been published...

**Root cause**: I hadn't generated a sitemap.xml.

**Fix**: Added `next-sitemap` to Next.js -- 2 days later, indexed pages jumped to 120.

**Lesson**: Don't neglect the basics.

---

### Failure 2: Credit Calculation Bug in chatweb.ai

Someone reported "I can use it for free way too much." I investigated and found that the credit calculation was using a **floor function**.

```rust
// Wrong
let credits = (tokens / 1000.0).floor() as i64;

// Correct
let credits = (tokens as f64 / 1000.0).ceil() as i64;
```

Whether 0.8 tokens or 1 token, it should cost **1 credit**, but the floor function was rounding down to 0.

We had to recalculate credits for past users and issue partial refunds.

**Lesson**: Test billing systems rigorously.

---

### Failure 3: Missed Support Ticket for jitsuflow

A dojo owner complained: "I sent an inquiry but never got a reply."

I looked into it and found the email had landed in **Gmail's spam folder**.

It went unnoticed for 2 weeks.

**Fix**: Migrated support email to Zendesk (automatic sorting).

**Lesson**: Manual operations have their limits.

---

## Key Takeaways

### 1. Start Small, Listen to Feedback

Every service grew through the cycle of "MVP -- feedback -- improvement."

Don't aim for perfection from the start.

### 2. Think About Monetization Early

Elio and webllm.app are "interesting" but generate zero revenue.

That's fine as a hobby. But for a business, figure out monetization early.

### 3. Automate Everything You Can

- Blog post social sharing -- Zapier
- Support -- Zendesk + AI
- Monthly reports -- Script-generated

Humans should focus on **the creative parts only**.

### 4. Share Your Failures Publicly

"The Time I Accidentally Racked Up 8.45 Million Yen in Charges" was my biggest viral hit.

Failure stories resonate with people.

### 5. Health Comes First

5.2 hours of sleep is not enough. Aiming for 6 hours next month.

---

## Goals for Next Month (March 2026)

### chatweb.ai

- [ ] Launch credit packs (1,000 credits = 1,000 yen)
- [ ] Improve Agentic Mode accuracy
- [ ] Build out Japanese documentation

### jitsuflow

- [ ] Increase video storage from 1GB to 5GB (Pro Plan only)
- [ ] Reduce support time to under 5 hours/week
- [ ] Complete English localization (preparation for international expansion)

### yukihamada.jp

- [ ] Grow newsletter signups to 50/month
- [ ] Publish 8 blog posts per month
- [ ] Achieve top 3 SEO rankings for 5 keywords

### news.online/xyz

- [ ] Automate article creation (build AI pipeline)
- [ ] Book 10 sponsored articles per month
- [ ] Grow LINE Bot subscribers to 500

### Elio

- [ ] Draft enterprise edition proposal
- [ ] Begin Android version development
- [ ] Finalize monetization model

---

## Closing Thoughts

Running 8 services simultaneously is, honestly, **chaos**.

But it's fun.

Every day is a learning experience. There are plenty of failures, but there's a real sense of moving forward.

I'll keep at it next month.

---

## Links

- [chatweb.ai](https://chatweb.ai)
- [jitsuflow.app](https://jitsuflow.app)
- [yukihamada.jp](https://yukihamada.jp)
- [news.online](https://news.online)
- [news.xyz](https://news.xyz)
- [Elio GitHub](https://github.com/yukihamada/elio)

---

## Related Posts

- [The Time I Accidentally Racked Up 8.45 Million Yen in Charges](/blog/845-man-yen)
- [How to Use AI at a Company That Banned ChatGPT](/blog/offline-ai-enterprise)
- [How I Digitally Transformed a BJJ Dojo for 120K Yen per Month](/blog/bjj-dojo-dx)

---

**Next report scheduled**: March 15, 2026

**Questions or comments**: Reach out on Twitter [@yukihamada](https://twitter.com/yukihamada)

---

**Tags**: `Monthly Report` `Startup` `Entrepreneurship` `SaaS` `AI` `Failure Stories` `Real Talk`
