---
title: "EnablerDAO - February 2026 Project Report"
date: "2026-02-14"
author: "Yuki Hamada"
tags: ["EnablerDAO", "Project Report", "Web3", "AI", "SaaS"]
description: "A summary of the latest status, tech stack, revenue model, and outlook for EnablerDAO's 12 projects."
image: "/blog/enablerdao-status-2026-02.png"
---

# EnablerDAO February 2026 Project Report

Hi, I'm Yuki Hamada. Here is a status report on the 12 projects currently in development at EnablerDAO.

## TL;DR

- **12 projects running** (6,000+ visits/month)
- **Current MRR**: JPY 280,000/month
- **12-month target**: JPY 3,000,000/month
- **Latest**: enablerdao.com migration to Cloudflare Pages complete

---

## Project Overview

### AI & Technology

#### 1. Chatweb.ai - AI-Powered Web Automation
Just give instructions via voice or text, and AI operates the browser to automate your tasks. Also available on LINE and Telegram.

- **Tech**: Rust Lambda, Next.js
- **Deploy**: AWS Lambda (ap-northeast-1)
- **Traffic**: 289 visits/month
- **Latest Feature**: Agentic Mode (multi-iteration tool execution)
- **Pricing**: Free / $9/month / $29/month

#### 2. Wisbee (Integrated with Chatweb.ai)
A privacy-focused AI assistant. Now integrated with Chatweb.ai, providing even more powerful features.

- **Traffic**: 613 visits/month
- **Integration Complete**: February 2026

#### 3. Elio Chat - Fully Offline AI on iPhone
An AI chat app that works without any network connection. All data is processed on-device, ensuring complete privacy.

- **Platform**: iOS (App Store)
- **Traffic**: 506 visits/month
- **Next Step**: Preparing for Product Hunt launch

#### 4. News.xyz - AI News Delivery
AI automatically collects and delivers news across multiple topics. Read the articles you care about, efficiently.

- **Traffic**: 506 visits/month
- **Next Step**: Preparing for Product Hunt launch

---

### Business Tools

#### 5. StayFlow - Short-Term Rental & Accommodation Management (Highest Traffic)
Centralized management of reservations, cleaning, and check-ins. Integrations with Airbnb and other platforms dramatically improve operational efficiency.

- **Traffic**: 1,840 visits/month -- **highest**
- **Tech**: Next.js, Supabase
- **Pricing**: From JPY 5,000/month

#### 6-9. Other SaaS Products
- **BANTO**: Invoice management for the construction industry (186 visits/month)
- **Totonos**: Automated corporate finance (103 visits/month)
- **VOLT**: Live auctions (205 visits/month)
- **Enabler**: Lifestyle services (107 visits/month)

---

### Security

#### 10. Security Scanner - Free Web Security Assessment
Enter a URL and get your website's security rated from A to F. Checks 8+ types of security headers.

- **Traffic**: 113 visits/month
- **Pricing**: Free / $19/month (Pro)

---

### Sports & Community

#### 11. JitsuFlow - Brazilian Jiu-Jitsu App
Streamline practice logging and dojo management. Visualize your technique acquisition progress.

- **Traffic**: 1,310 visits/month
- **Deploy**: Fly.io (nrt)
- **Tech**: Next.js, Supabase

---

## Tech Stack and Philosophy

### Why We Chose These Technologies

#### Edge-First Architecture
Leveraging Cloudflare Pages, Workers, and Fly.io for low-latency, global delivery.

#### TypeScript + Rust
- **TypeScript**: Type safety across frontend and backend
- **Rust**: Fast execution and low cost for Lambda functions

#### Supabase
An open-source Firebase alternative. PostgreSQL + real-time subscriptions.

---

## Revenue Model

### Subscriptions (Primary)
- Chatweb.ai Pro: $9-29/month
- Elio Chat Pro: $4.99/month
- StayFlow: JPY 5,000-50,000/month
- Security Scanner Pro: $19/month

### Transactions
- VOLT: 10% transaction fee
- Enabler: 15% marketplace fee

### Current Status and Targets

| Period | MRR | Paying Users |
|--------|-----|--------------|
| **Current** | JPY 280,000 | 20 |
| **3 months** | JPY 500,000 | 100 |
| **6 months** | JPY 1,500,000 | 300 |
| **12 months** | JPY 3,000,000 | 1,600 |

---

## Major Recent Updates (February 2026)

### enablerdao.com Redesign

1. **Migration to Cloudflare Pages**
   - Edge Runtime support
   - Global CDN across 330 locations
   - Operates within the free tier

2. **Conversion Optimization**
   - Added newsletter CTA
   - Enhanced product cards (displaying pricing and user counts)
   - Improved hero CTA ("Get Started for Free")
   - Trust badges (12 products / 6,000+ users)

3. **Technical Improvements**
   - All API routes now support Edge Runtime
   - Migrated to Web Crypto API (eliminated Node.js dependency)
   - Configured GitHub Actions CI/CD

---

## Operating as a DAO

### EBR Token
EnablerDAO is governed by a **voting token (EBR)**.

- **Total Supply**: 1,000,000 EBR
- **How to Earn**: Code contributions, bug reports, documentation
- **Purpose**: Voting on project direction

It is not an investment product -- it is a **governance tool**.

---

## Next Steps

### Short Term (Within 1 Month)
- [ ] Complete enablerdao.com deployment
- [ ] Finalize totonos.jp strategy
- [ ] Launch news.xyz on Product Hunt
- [ ] Submit elio.love to the App Store

### Medium Term (Within 3 Months)
- [ ] Start ad campaigns (Reddit/Google/Apple)
- [ ] Reach 1,000 newsletter subscribers
- [ ] Surpass 100 paying users

### Long Term (Within 12 Months)
- [ ] Reach MRR of JPY 3,000,000
- [ ] Hire a team of 5
- [ ] Grow the DAO community to 1,000 members

---

## How to Get Involved

EnablerDAO is an open organization that anyone can join.

1. **Try it out first**: Use free tools like [Security Scanner](https://chatnews.tech)
2. **Give feedback**: Suggest improvements on [GitHub](https://github.com/yukihamada)
3. **Contribute**: Earn EBR tokens through code, documentation, and bug reports
4. **Vote**: Help decide the direction of projects

---

## Summary

While developing and operating 12 projects in parallel, we have reached 6,000 monthly visits and an MRR of JPY 280,000.

Our next goal is **MRR of JPY 3,000,000 within 12 months**.

We aim for sustainable growth built on three pillars: technology choices (Edge-First, TypeScript, Rust), revenue model (subscriptions + transactions), and DAO governance (EBR token).

If you are interested, please visit [enablerdao.com](https://enablerdao.com).

---

**Yuki Hamada** / [yukihamada.jp](https://yukihamada.jp)
EnablerDAO Founder
GitHub: [@yukihamada](https://github.com/yukihamada)
X: [@yukihamada](https://x.com/yukihamada)
