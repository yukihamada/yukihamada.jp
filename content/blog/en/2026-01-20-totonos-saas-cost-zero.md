---
title: "From 50,000 Yen a Month in SaaS Fees to Zero -- How Totonos Streamlined Our Operations"
date: 2026-01-20
tags: [Totonos, SaaS, AI, EnablerDAO]
description: "We consolidated 8 SaaS tools into one with Totonos, then added barcode inventory management, LINE/Slack notifications, automatic email ingestion, and AI agents. Monthly cost went from 50,000 yen to nearly zero."
---

# From 50,000 Yen a Month in SaaS Fees to Zero. One Week of Streamlining Operations with Totonos

## Sound Familiar?

- You are using multiple SaaS tools and monthly fees are piling up
- You have thought, "It would be so convenient if I got notifications on LINE"
- Your inventory management is still stuck in Excel, and it makes you nervous
- You are tired of manually transcribing invoice emails
- You have no record of "who did what," and it worries you

**If even one of these applies to you, this article is for you.**

---

## TL;DR

**We added even more features to Totonos, which already consolidated 8 SaaS tools into one.**

- Scan a barcode with your phone and inventory is updated instantly
- Get automatic payment deadline notifications via LINE and Slack
- Forward an invoice email and it is recorded automatically
- Display only the features each company needs
- Verified with 675 test items for peace of mind

**And the monthly cost dropped from roughly 50,000 yen to nearly zero.**

---

## Cost Comparison: Saving Up to 600,000 Yen Per Year

### Traditional SaaS vs. Totonos

| Category | Traditional (est. monthly) | Totonos |
|-------------|-----------------|---------|
| CRM | 15,000 yen | Included |
| Accounting software | 5,000 yen | Included |
| Invoice generation | 1,000 yen | Included |
| Contract management | 10,000 yen | Included |
| Attendance & HR | 10,000 yen | Included |
| Inventory management | 5,000 yen | Included |
| LINE notifications | 3,000 yen | Included |
| Slack integration | 2,000 yen | Included |
| **Total** | **51,000 yen/month** | **0 to 5,000 yen/month** |

Savings of up to 600,000 yen per year. Since Totonos can run on your own server, you only need to pay for infrastructure.

---

## New Features

We spent **an additional 21 hours** adding the following features to the Totonos we built in 29 hours.

### Inventory Management: Scan and Update from Your Phone

**"Inventory is the one thing still stuck in Excel..." -- This fixes that.**

It is as simple as three steps:

1. Tap the "Scan" button on your phone
2. Point the camera at the product barcode
3. Adjust the quantity and you are done!

Supported barcodes: JAN codes (Japanese products), UPC (international products), and other major standards.

Automatic reading of delivery slips is also supported. Just photograph a paper delivery slip with your phone, and AI automatically recognizes the product name, quantity, and unit price.

There is also an automatic reorder alert. Set a threshold for each product -- "reorder when stock drops to X units" -- and you get notified automatically when inventory runs low. Purchase orders can be created with a single click.

### LINE & Slack Notifications: Never Miss What Matters

Examples of notifications you can receive on LINE:

- "Invoice #INV-042 is due in 3 days"
- "Product A is down to 5 units in stock"
- "A new lead has been registered"
- "A contract has been signed"

Setup takes just three steps. Add the Totonos official LINE account as a friend, enter the linking code, choose which notifications you want to receive, and you are done. Google Chat is also supported.

### Automatic Email Ingestion: Just Forward and It Is Recorded

1. Get a dedicated email address from Totonos
2. Forward your invoice emails to that address
3. AI analyzes the content and registers it automatically

Invoices are automatically added to payment management. Contracts are saved to document management. Inquiries are automatically registered as leads.

### Customization: Show Only the Features You Need

Choose which modules to display for each company.

- Manufacturing: Focus on inventory management and purchasing
- Service industry: Focus on customer management and invoicing
- Small teams: Keep it simple so nobody gets lost

### AI Agents: Automate Your Operations

Available agents:

- **Invoice Reminder** -- Automatically notifies you when invoice due dates are approaching
- **Expense Auto-Processing** -- Registers expenses automatically from receipt images
- **Lead Scoring** -- Automatically prioritizes prospects
- **Contract Expiry Monitor** -- Watches contract renewal dates and sends notifications
- **Email Auto-Classification** -- Automatically categorizes and processes incoming emails
- **Sales Forecasting** -- Uses historical data to generate AI-powered sales predictions

---

## Why You Can Use It with Confidence

### Verified with 675 Test Items

Display and interaction checked on every screen. Confirmed that data creation, updates, and deletions work correctly. Automated checks run with every update.

### Social Login Support

One-click login with your Google account, GitHub account login, and passwordless login via email link. Secure OAuth 2.0 authentication implemented with Supabase Auth.

### Full Activity Logging

Ready for internal controls and audits. Who did what, and when -- every action is automatically recorded.

---

## Summary: From Zero to Usable in 50 Hours

### What Changed in One Week

| Aspect | Before | After |
|-----|--------|-------|
| Inventory management | None | Barcode scanning & auto-reorder |
| Notifications | None | LINE, Slack, and email |
| Quality | Unverified | 675 test items verified |
| Customization | None | Configurable per company |

### The "Totonos" Concept

Totonos takes its name from the Japanese word "totonou," meaning to put things in order.

It **consolidates** scattered SaaS tools into one, **automates** tedious tasks, and **organizes** messy information. Totonos helps your operations get in order.

### Development Statistics

| Metric | As of 1/14 | As of 1/20 | Change |
|------|----------|----------|------|
| Lines of code | 64,174 | 110,751 | +72% |
| Tests | 0 | 675 | New |
| E2E tests | 0 | 36 | New |
| Development time | ~29 hours | ~50 hours | +21 hours |

### Tech Stack

| Layer | Technology |
|---------|------|
| Frontend | React 18, TypeScript, Tailwind CSS, shadcn/ui |
| Backend | Supabase (PostgreSQL, Auth, Storage, Edge Functions) |
| AI | Claude 3.5 Sonnet (conversation, OCR, analysis) |
| Payments | Stripe (subscriptions, credit billing) |
| Email | Resend (sending & receiving) |
| Notifications | LINE Messaging API, Slack Webhook |

---

**Totonos** -- A business OS that consolidates 8 SaaS tools into one
https://totonos.jp
