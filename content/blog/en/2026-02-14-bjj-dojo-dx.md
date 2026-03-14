---
title: "How I Digitally Transformed a BJJ Dojo for 120K Yen per Month"
date: 2026-02-14
tags: [BJJ, SaaS, DX, Flutter]
description: "Paper rosters, handwritten attendance, cash-only tuition. I took a BJJ dojo stuck in an analog nightmare and fully digitized it."
---

# How I Digitally Transformed a BJJ Dojo for 120K Yen per Month

## Prologue: The Analog Nightmare

In 2019, I was training at a BJJ dojo in Setagaya, Tokyo.

The training was great. The teammates were great. The instructor was great.

But the **operations were chaos**.

### The Analog Nightmare in Practice

1. **Member roster**: An Excel file (on the instructor's PC, no backups)
2. **Attendance**: Handwritten paper notebook
3. **Tuition**: Cash only, handwritten receipts
4. **Schedule**: Handwritten on a whiteboard
5. **Communication**: Multiple LINE groups (total chaos)

One day, the instructor said:

"I've lost track of who paid tuition this month."

...Completely unacceptable.

## Chapter 1: Defining the Problems

### The Instructor's Pain Points

- **Time-consuming**: 1-2 hours of administrative work every day
- **Error-prone**: Handwriting is illegible, data entry mistakes everywhere
- **Financial chaos**: No way to tell who has paid
- **Missed messages**: Multiple LINE groups causing confusion

### The Members' Pain Points

- **Schedule unknown**: Had to check the whiteboard in person
- **Paying tuition is a hassle**: Bringing cash every time
- **No idea who's coming**: Can't tell in advance who your training partners will be
- **Belt progress unclear**: How many more sessions until the next stripe?

### My Thought

"Couldn't an app solve all of this?"

---

## Chapter 2: Building the MVP

### Initial Design (November 2019)

**Minimum required features**:

1. Member registration and login
2. Schedule display
3. Attendance tracking
4. Tuition management (tracking who has paid)

**Tech stack**:

- Frontend: Flutter (iOS + Android)
- Backend: Firebase (Authentication, Firestore, Cloud Functions)
- Language: Dart

I chose Flutter because **one codebase covers both iOS and Android**.

### Development Period: 3 Weeks

- Week 1: Design and UI creation
- Week 2: Firebase integration
- Week 3: Testing and debugging

Every evening, 2-3 hours of development after training.

### First Release (December 2019)

I showed it to the dojo instructor.

"Oh, this is great!"

We started a test run with 20 members.

---

## Chapter 3: Feedback and Iteration

### Requests from Members (First Month)

1. **Profile photos**: Can't tell who is who
2. **Belt progress**: How many sessions until the next stripe?
3. **Training log**: Want to know how many times I've trained
4. **Sparring partner selection**: I want to train with this person
5. **Notifications**: Notify me of schedule changes

### Requests from the Instructor

1. **Automatic tuition reminders**: Notify members with unpaid tuition
2. **Reporting**: Monthly attendance rates and revenue
3. **QR code attendance**: Just scan your phone to check in
4. **Belt management**: Record stripes and promotions

### v2.0 Release (February 2020)

All of the above features implemented.

Development time: 2 hours per night x 30 days = approximately 60 hours

---

## Chapter 4: Inquiries from Other Dojos

### Word of Mouth Spreads

A member posted on social media:

"Our dojo uses an app for attendance. It's incredibly convenient."

Other dojos started reaching out:

"We want to use it too!"

### Multi-Dojo Support Development (May-August 2020)

**Challenge**: Every dojo runs differently

- Dojo A: Monthly tuition model
- Dojo B: Ticket model (10-session packs)
- Dojo C: Drop-in (pay per visit)

We built a **flexible billing system** to accommodate all of them.

### The Decision to Go SaaS

"Giving it away for free isn't sustainable."

- Server costs: 30,000 yen/month (Firebase)
- Support work: 10 hours/week

We decided to offer it as a **monthly subscription SaaS**.

Pricing:

- **Basic**: 12,000 yen/month (up to 100 members)
- **Pro**: 24,000 yen/month (up to 300 members)
- **Enterprise**: 48,000 yen/month (unlimited members)

---

## Chapter 5: The Birth of "jitsuflow"

### The Name

- **jitsu**: Jiu-Jitsu
- **flow**: Smooth operations

**jitsuflow.app**

Domain registration: $12/year (cheap!)

### Logo Design

Outsourced on Fiverr: $50

A simple wave design in BJJ's signature colors (blue and black).

---

## Chapter 6: Growth Hacking

### Early User Acquisition (September-December 2020)

1. **Reddit post**: Posted on r/bjj -- 150 upvotes
2. **Facebook ads**: Targeted BJJ-related groups -- spent $500
3. **YouTube sponsors**: Provided free accounts to BJJ instructors -- review videos

Results:

- Registered dojos: 32
- Members: 890
- MRR: 380,000 yen

### The Power of Word of Mouth (2021)

"jitsuflow is seriously convenient" spread through the BJJ community.

- Registered dojos: 120
- Members: 3,200
- MRR: 1.44 million yen

---

## Chapter 7: Current Status (February 2026)

### Numbers

- **Registered dojos**: 85 (peaked at 120, declined during COVID, recovering)
- **Members**: 1,300
- **MRR**: 1.02 million yen
- **Monthly active users**: 980

### Key Features (2026 Edition)

1. **Schedule management**: Class booking and cancellation
2. **Attendance tracking**: QR code and NFC tap support
3. **Tuition management**: Stripe integration, automatic billing
4. **Belt management**: Stripe and promotion records
5. **Training log**: Personal dashboard
6. **Video library**: Save and share technique videos
7. **Tournament management**: Create competition brackets
8. **Community**: In-dojo social network

### Tech Stack (2026 Edition)

- Frontend: Flutter 3.x
- Backend: Firebase + Supabase (partial migration underway)
- Payments: Stripe
- Video: Cloudflare Stream
- Notifications: Firebase Cloud Messaging
- Analytics: Mixpanel

---

## Chapter 8: Cost Structure and ROI

### Monthly Costs (February 2026)

| Item | Amount |
|------|--------|
| Firebase | 32,000 yen |
| Supabase | 25,000 yen |
| Stripe fees | 36,000 yen (3.5% of MRR) |
| Cloudflare Stream | 18,000 yen |
| Domain / SSL | 1,000 yen |
| Mixpanel | 12,000 yen |
| **Total** | **124,000 yen** |

### Revenue

| Plan | Dojos | Unit Price | Subtotal |
|------|-------|------------|----------|
| Basic | 45 | 12,000 yen | 540,000 yen |
| Pro | 32 | 24,000 yen | 768,000 yen |
| Enterprise | 8 | 48,000 yen | 384,000 yen |
| **Total** | **85** | - | **1,692,000 yen** |

(Note: Actual MRR is 1.02 million yen. The above is list price, before discounts and free trial periods.)

### Profit

- **MRR**: 1.02 million yen
- **Costs**: 124,000 yen
- **Gross profit**: **896,000 yen**

(Development time not included. I run everything solo.)

---

## Chapter 9: Failures and Lessons

### Failure 1: Feature Creep (2021)

Added "this and that" -- UI became overly complex -- churn rate increased.

**Lesson**: Simple wins. Remove unused features.

### Failure 2: Manual Support Was Unsustainable (2022)

Spending 20 hours a week on support inquiries -- nearly burned out.

**Lesson**: Introduced FAQ and a chatbot -- reduced to 5 hours/week.

### Failure 3: Pricing Was Too Low (Early Days)

Basic at 5,000 yen/month -- too cheap to cover support costs.

**Lesson**: Raised prices in 2021 (5,000 yen to 12,000 yen) -- churn rate barely changed.

---

## Chapter 10: User Testimonials

### Dojo Owner A (40s, Tokyo)

"Thanks to jitsuflow, my administrative work dropped from 10 hours to 2 hours a week.
I can now focus on training and teaching."

### Member B (30s, Osaka)

"Since I can check the schedule on the app, it's easy to drop in after work.
Being able to see my belt progress keeps me motivated."

### Dojo Owner C (50s, Fukuoka)

"When membership dropped during COVID, the online class booking feature was a lifesaver.
The Zoom integration made it easy to run online sessions."

---

## Chapter 11: What's Next

### Roadmap (2026)

1. **AI features**:
   - Attendance prediction ("How many people will come tomorrow?")
   - Belt promotion timing prediction
   - Personalized training menu suggestions

2. **Global expansion**:
   - English and Portuguese support (targeting the Brazilian market)
   - Multi-currency payment support

3. **Expanding to other martial arts**:
   - Judo, karate, Muay Thai, etc.

### Long-Term Vision

"Connect every dojo in the world through jitsuflow."

There are currently around 100,000 BJJ dojos worldwide.

12,000 yen/month x 100,000 dojos = **1.2 billion yen/month**

...Dream big.

---

## Summary: DX Isn't Just for Big Companies

jitsuflow took a dojo that relied on nothing but paper notebooks and **fully digitized it for 120,000 yen per month in operating costs**.

### Benefits of DX

1. **Time savings**: Administrative work reduced from 10 hours/week to 2 hours
2. **Revenue growth**: Reduced unpaid tuition, increased new memberships
3. **Member satisfaction**: Visibility into schedules and progress

### All You Need Is the Decision

- Programming experience? Not required (you can outsource)
- Budget? Starting at 120,000 yen/month
- Time? 3 months initially, then 5 hours/month

**Start small. Listen to feedback. Iterate.**

That's all it takes to escape the analog nightmare.

---

## Links

- **jitsuflow official**: https://jitsuflow.app
- **GitHub**: https://github.com/yukihamada/jitsuflow
- **Contact**: info@jitsuflow.app

---

## Related Services

- [chatweb.ai](https://chatweb.ai) - AI agent platform
- [Elio](https://github.com/yukihamada/elio) - Privacy-focused local AI
- [yukihamada.jp](https://yukihamada.jp) - Portfolio

---

## Related Posts

- [The Time I Accidentally Racked Up 8.45 Million Yen in Charges](/blog/845-man-yen)
- [How to Use AI at a Company That Banned ChatGPT](/blog/offline-ai-enterprise)
- [February 2026 Monthly Report](/blog/2026-02-report)

---

**Tags**: `BJJ` `SaaS` `DX` `Flutter` `Firebase` `Startup` `Dojo Management`
