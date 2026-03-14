---
title: A Real-World Account of Prompt-Driven Development — Running 6 Products with 5 Prompts in a Single Day
date: 2026-02-23
tags: [ai, claude-code, prompt-engineering, building-in-public]
description: A complete mapping of 5 actual prompts and their execution results. The only human work is deciding what to build.
---

## What Is Prompt-Driven Development?

A development methodology where you simply give natural language instructions to AI coding tools like Claude Code or OpenClaw, and the code gets written, tested, and deployed.

The traditional waterfall process (requirements → design → implementation → testing → deployment) is **condensed into a single prompt**.

The human's job is limited to deciding "what to build" and performing the final review.

---

## 5 Actual Prompts and Their Results

### Prompt 1: Cross-Product Sprint

```
Finish all the software, test it,
hold a meeting, update the blog, and run another sprint.
```

**What the AI did:**
- Analyzed the current state of 6 products (StayFlow, StayFlowApp, MisebanAI, JitsuFlow, IronClaw, chatweb.ai)
- Scanned todo.md / git status across all projects
- Output the sprint plan as a blog post
- Committed 17,829 lines of uncommitted changes
- Wrote the AI-Driven Development Manifesto for the CTO

### Prompt 2: CTO Briefing + Blog Operations

```
Basically we'll be writing code super efficiently using claude code,
openclaw, etc., so I want the CTO properly briefed on that.
Of course tests are needed, but we don't need minor debates
like whether to add dark mode or not.
Put an explanation at the top of the blog about how we'll run it
going forward so it's easy to update from here on.
```

**What the AI did:**
- Added a permanent guide section at the top of the blog list page
- Wrote the "AI-Driven Development Manifesto" article: tool stack, decision criteria, do/don't table
- Created a status table for all products

### Prompt 3: Domain Setup

```
Got the misebanai.com domain and added it to Cloudflare.
Make it accessible. And include that in the cycle too.
```

**What the AI did:**
- Deployed the app to Fly.io (nginx, 23MB image)
- Issued SSL certificates (misebanai.com + www)
- Bulk-replaced the domain across all 28 files (miseban.ai → misebanai.com)
- Updated sitemap.xml
- Created blog post #021 + updated blog/index.html
- Committed → pushed to GitHub → redeployed

### Prompt 4: Grandma-Friendly UX

```
Whether it's a PC, Raspberry Pi, or ESP-32, I want it to just
connect on startup, or at most just ask for a password.
Make it simple to connect and use. Easy enough for a grandma.
```

**What the AI did:**
- Built a network camera auto-discovery module (scanner.rs)
  - Parallel scan of local subnet /24
  - Probing RTSP (554, 8554) and HTTP (80, 8080)
  - JPEG magic byte verification
- Built a WebUI setup wizard (setup.rs)
  - Launched an axum server on port 3939
  - 6-digit pairing code entry screen
  - Camera selection → automatic config.toml generation
- Designed a systemd service file + install script

### Prompt 5: Meta-Article

```
Blog about what prompts I've been using to drive all this so far.
```

**What the AI did:**
- Wrote this very article (recursive)

---

## Results in Numbers

| Metric | Value |
|--------|-------|
| Prompts typed by a human | 5 (estimated 3 minutes total) |
| Blog posts generated | 5 |
| Files changed | 70+ |
| Lines of code committed | 17,829+ |
| Deployments | 3 |
| New feature modules | 3 (scanner, setup, sprint plan) |
| Cross-project analysis | 6 products |

---

## Lessons Learned

1. **Prompts can be short** — If the intent is clear, one line is enough. The AI translates it into a concrete design.
2. **"Easy enough for a grandma" is a valid spec** — Even abstract requirements get expanded by the AI into concrete solutions like zero-config, auto-discovery, and WebUI.
3. **Test and deployment automation is a given** — AI-written code and human-written code are held to the same quality standard.
4. **Blog = development log** — Write one every sprint and documentation accumulates automatically. It also serves as external communication.
5. **Eliminate trivial debates** — Instead of spending 30 minutes debating dark mode, ship 3 features in those 30 minutes.

---

**Next cycle**: Finish the zero-config agent, YOLO inference pipeline, and start beta recruitment.
