---
title: "Building an ATP Supplement Guide — Scoring Algorithm and Design Philosophy"
date: 2026-02-24
tags: [Health, JavaScript, サプリメント]
description: "How I built a server-free, personalized supplement recommendation engine that takes your profile and outputs evidence-based priorities. A walkthrough of the scoring algorithm design."
---

## Introduction

"Which supplements should I actually be taking?"

Search online and the sheer volume of information is paralyzing. Research papers are in English and hard to digest; affiliate articles are of questionable trustworthiness. So I decided to build my own tool -- one where **you enter your profile and get evidence-based, prioritized recommendations**. That was the motivation.

The result is the "ATP Production + Longevity Supplement Complete Guide." It runs from a single HTML file with no server required -- a fully client-side personalized recommendation engine.

---

## Overall Structure

Technically, it is quite simple:

- **Single HTML file** (CSS + JS all inline)
- No server-side processing. Personal data is never sent anywhere
- The database is a JavaScript array literal (`ALL_ITEMS`)

It holds data for 18 supplements and 12 dietary sources -- 30 items total -- entirely on the client side. Based on user input, it scores, sorts, and displays the results.

---

## 7-Step Onboarding

Rather than jumping straight into calculations, the user answers 7 questions in sequence:

| Step | Question | Purpose |
|---|---|---|
| 1 | What is your priority? (ATP / longevity / exercise / beauty) | Adjusts score weighting |
| 2 | Physical concerns (joints / fatigue / sleep / focus / skin) | Adds score to specific items |
| 3 | Fish consumption frequency | Adjusts omega-3 item priority |
| 4 | Meat and egg consumption frequency | Adjusts carnitine and B-vitamin priority |
| 5 | Fermented food and vegetable consumption frequency | Adjusts spermidine and related item priority |
| 6 | Supplements currently being taken | Lowers priority of duplicate items |
| 7 | Current medications | Displays drug interaction warning badges |

The key design principle here is that **every answer directly feeds into the scoring logic**. There are zero "just asking for the sake of asking" questions.

---

## How Scoring Works

This is the core of the tool.

### Fixed Scores for Each Item

Every item has pre-assigned scores across 4 axes (each out of 10):

- **atp**: Direct contribution to ATP synthesis
- **longevity**: Longevity and anti-aging effects
- **exercise**: Impact on exercise performance
- **evidence**: Strength of scientific evidence

For example, creatine is scored as `{ atp: 10, longevity: 5, exercise: 10, evidence: 10 }` -- top marks for ATP synthesis and exercise, with the strongest evidence rating.

### Dynamic Weighting

Base weights are determined as follows:

- ATP weight = 0.35 (fixed base)
- Exercise weight = 0.30 if exercising 3+ times/week, 0.15 otherwise
- Longevity weight = 0.30 if age 40+, 0.20 otherwise
- Evidence weight = remainder (1.0 minus the sum of the above three)

In other words, **weights change based solely on age and exercise frequency**. A 40+ person with regular exercise habits gets "ATP + exercise + longevity" as their three pillars, while a sedentary person in their 20s gets a stronger emphasis on "ATP + evidence."

### Diet-Based Score Adjustments

"Eats fish 3+ times per week" applies a **-1.5 point** penalty to omega-3 items (already well-covered by diet). Conversely, "rarely eats fish" applies a **+1.0 point** boost (dietary deficiency increases priority).

This produces realistic recommendations: "no need to supplement what your diet already covers; prioritize what it lacks."

### Drug Interaction Warnings

These do not affect scores but are displayed as **warning badges**:

- Anticoagulants -> Omega-3 (bleeding risk), D3+K2 (warfarin attenuation)
- Diabetes medications -> R-alpha-lipoic acid (hypoglycemia risk)
- Antihypertensives -> Magnesium (excessive blood pressure drop)
- Immunosuppressants -> "Consult your doctor" warning on **all supplements**

---

## Two-Mode Design

### Standard Mode

Displays **fixed doses** based on RCTs (randomized controlled trials) and clinical guidelines. For example, "CoQ10 -> 100-200mg/day" -- established dosages independent of body weight.

### Experimental Mode

Includes **variable doses** calculated from body weight. For CoQ10, this would be `body weight x 1.5-3mg/kg`. A person weighing 62kg would see "93-186mg/day."

In experimental mode, each item's 4-axis scores are visualized as bar graphs, making it immediately clear "why this item ranks where it does."

---

## UI/UX Details

### Glassmorphism

The input sections use a translucent glass-style design with `backdrop-filter: blur()`. The gradient background in the hero area shows through, creating a sense of depth.

### Card Entrance Animations

Result cards fade in from below using CSS `@keyframes cardEntrance`, with staggered delays applied via CSS variables. Higher-ranked cards appear first, visually reinforcing the ranking.

### Mobile-First

The responsive layout uses a `min-width` approach. All touch targets are at least 44px (per Apple HIG guidelines).

---

## Summary

The design philosophy behind this tool rests on three pillars:

1. **Transparency** -- Explain every recommendation through scores and underlying mechanisms
2. **Personalization** -- Factor in age, weight, exercise habits, diet, existing supplements, and medications
3. **Safety** -- Drug interaction warnings, standard/experimental mode separation, and explicit "consult your doctor" prompts

The goal is not to produce "the ultimate supplement list" but to deliver "priorities tailored to you." That is what this tool set out to achieve.
