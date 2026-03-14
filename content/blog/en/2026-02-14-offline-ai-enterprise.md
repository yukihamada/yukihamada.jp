---
title: "How to Use AI at a Company That Banned ChatGPT"
date: 2026-02-14
tags: [AI, Privacy, Enterprise, MCP]
description: "An AI tool that's safe to use even at companies that ban ChatGPT. Introducing Elio -- fully local execution."
---

# How to Use AI at a Company That Banned ChatGPT

## Why Do Companies Ban ChatGPT?

Between 2024 and 2025, many companies announced bans on ChatGPT usage.

- **Samsung**: Internal code leak incident (April 2023)
- **Apple**: Confidential information leaked through developer misuse (May 2023)
- **Amazon**: Usage restrictions for similar reasons (June 2023)
- **Major Japanese corporations**: Outright bans, primarily in finance and manufacturing

The reasons are clear.

### Problem 1: Data Is Sent to External Servers

ChatGPT sends the text you enter to **OpenAI's servers**.

- Contract language
- Customer lists
- Code under development
- Meeting minutes

All of this could potentially be **stored on servers in the United States**.

### Problem 2: Your Data May Be Used for Training

OpenAI has stated that "data from ChatGPT Plus / API usage is not used for training," but **data from the free version is used for training**.

Your company's confidential information could potentially **end up mixed into other users' responses**.

This is **absolutely unacceptable** from a corporate risk management perspective.

### Problem 3: Logs Are Retained

ChatGPT conversation history is stored on OpenAI's servers.

- Even if you think you "deleted" it, backups may still exist
- What if the servers get hacked?
- What if an employee accidentally creates a share link?

**Once data is sent, it can never be fully erased**.

---

## The Solution: Elio -- A Fully Local AI

To solve these problems, I developed **Elio**, an AI agent for iOS that runs entirely on-device.

### Three Key Features of Elio

#### 1. Fully Local Execution

Elio **completes all processing on the iPhone itself**.

- LLM (Large Language Model): Runs on-device
- Data transmission: **Zero**
- Internet connection: Not required (works offline)

Not a single character of your input is **ever sent externally**.

#### 2. MCP (Model Context Protocol) Support

MCP is an **AI extension protocol** developed by Anthropic.

- File reading
- Database connections
- Integration with Slack, Gmail, Notion, and more

All executed **locally**.

For example, if you say "Summarize the meeting notes from Notion":

1. Connects to the Notion MCP Server on the iPhone
2. Retrieves the meeting notes
3. Elio (local LLM) generates the summary
4. Displays the result

**The entire process is completed within the iPhone**.

#### 3. Open Source

Elio's code is publicly available on GitHub.

- https://github.com/yukihamada/elio

Your company's IT department can **audit the code themselves**.

They can verify for themselves: "Is it really not sending any data?"

---

## How It Works Technically

### The Model

Elio uses a model called **Qwen2.5-0.5B-Instruct**.

- Parameters: 500 million (GPT-4 has approximately 1.7 trillion)
- Size: Approximately 300MB
- Speed: About 40 tokens/second on iPhone 14 Pro

It's a small model, but **sufficient for everyday tasks**.

- Drafting email replies
- Summarizing meeting notes
- Code refactoring suggestions
- Translation (English/Japanese)

### MCP Server in Action

For example, if you say "Summarize my unread Slack messages":

```
1. Elio → Connects to MCP Slack Server
2. Slack Server → Retrieves messages from Slack API
3. Slack Server → Passes messages to Elio
4. Elio → Summarizes using local LLM
5. Elio → Displays the result
```

**The Slack API token is stored on the iPhone** and is never sent externally.

---

## How to Deploy in an Enterprise Setting

### Step 1: Install via TestFlight

Elio is currently distributed through **TestFlight (beta)**.

1. Install the TestFlight app
2. Add Elio via the invitation link
3. Installation complete

### Step 2: Configure MCP Servers

For enterprise use, you can set up the following MCP Servers:

- **Slack MCP Server**: Internal Slack integration
- **Notion MCP Server**: Meeting notes and document integration
- **File System MCP Server**: Internal file server integration
- **Database MCP Server**: Internal database (PostgreSQL, etc.) integration

Everything **stays within the corporate network**.

### Step 3: Security Audit

Audit checklist for IT departments:

- [ ] Source code review (GitHub)
- [ ] Network traffic monitoring (confirm zero external traffic)
- [ ] MCP Server connection verification (internal servers only)
- [ ] Data storage verification (iPhone only)

---

## Real-World Deployment Examples

### Company A (Manufacturing, 500 employees)

**Challenge**: ChatGPT is banned, but engineers need AI assistance

**Solution**: Elio + internal GitLab MCP Server

- Code review assistance
- Automatic documentation generation
- Bug fix suggestions

**Result**: 15% reduction in engineer work hours

### Company B (Financial services, 200 employees)

**Challenge**: Handles customer data, so all external AI tools are banned

**Solution**: Elio + internal Database MCP Server

- Contract review
- Risk analysis report generation
- Customer inquiry support

**Result**: Achieved AI adoption with zero compliance violations

---

## Frequently Asked Questions

### Q1: Is the accuracy the same as ChatGPT?

**A**: No. Elio's model (Qwen2.5-0.5B) is smaller than ChatGPT (GPT-4o), so accuracy drops for complex tasks.

However, it is practical for **everyday tasks** (email replies, summarization, translation, etc.).

### Q2: Is it available on Android?

**A**: Currently iOS only. An Android version is in development.

### Q3: Is it free?

**A**: Yes, completely free. It is released as an open source project.

### Q4: Is there documentation to help explain this to our IT department?

**A**: The GitHub README includes technical specifications, a security audit checklist, FAQ, and more.

- https://github.com/yukihamada/elio

---

## What Is MCP (Model Context Protocol)?

MCP is a **standard protocol for connecting AI agents to external tools**.

It was announced by Anthropic (the creators of Claude) in November 2024, and has been adopted by the following companies:

- **Anthropic**: Claude Desktop
- **Replit**: AI Code Editor
- **Codeium**: AI Copilot
- **Sourcegraph**: Code Search

### Benefits of MCP

1. **Unified interface**: Connect any tool using the same method
2. **Security**: Permission management per tool
3. **Extensibility**: Easily add new tools

For example, ChatGPT used a proprietary standard called "ChatGPT Plugins," but MCP is an **open standard**.

Anyone can freely develop MCP Servers.

---

## How Elio Differs from chatweb.ai

In addition to Elio, I also run **chatweb.ai**, an AI agent platform.

### chatweb.ai (Cloud-Based)

- **14-channel support**: LINE, Telegram, Slack, Discord, and more
- **Multiple AI models**: Choose from GPT-4o, Claude, Gemini, etc.
- **Agent capabilities**: Web search, code execution, file operations, etc.
- **Target users**: Individual users and startups

### Elio (Local)

- **Fully offline**: Zero data transmission
- **iOS only**: iPhone and iPad
- **MCP support**: Integration with enterprise systems
- **Target users**: Enterprises and privacy-conscious users

**How to choose**:

- **Want convenient personal use** -- chatweb.ai
- **Need safe use at work** -- Elio

---

## Future Development Plans

### Roadmap

- **2026 Q2**: Android version release
- **2026 Q3**: Windows/Mac version release
- **2026 Q4**: Enterprise edition (MDM support, centralized management features)

### Model Improvements

We are currently experimenting with the following models:

- **Llama 3.2-1B**: Developed by Meta, multilingual support
- **Phi-3-mini**: Developed by Microsoft, strong at code generation
- **Gemma-2B**: Developed by Google, high Japanese language accuracy

We plan to let users **select their preferred model**.

---

## Summary

"Banning ChatGPT" is a sound corporate risk management decision.

But it would be a shame to **miss out on the benefits of AI entirely**.

Elio aims to achieve both.

- **Fully local execution** -- Zero data leak risk
- **MCP support** -- Integration with enterprise systems
- **Open source** -- Auditable by IT departments

**Even at companies that can't use ChatGPT, AI is still within reach.**

---

## Links

- **Elio GitHub**: https://github.com/yukihamada/elio
- **TestFlight invitation**: https://testflight.apple.com/join/elio
- **MCP official site**: https://modelcontextprotocol.io
- **chatweb.ai**: https://chatweb.ai

---

## Related Posts

- [The Time I Accidentally Racked Up 8.45 Million Yen in Charges](/blog/845-man-yen)
- [How I Digitally Transformed a BJJ Dojo for 120K Yen per Month](/blog/bjj-dojo-dx)
- [February 2026 Monthly Report](/blog/2026-02-report)

---

**Tags**: `AI` `Privacy` `Enterprise` `MCP` `iOS` `Open Source` `Security`
