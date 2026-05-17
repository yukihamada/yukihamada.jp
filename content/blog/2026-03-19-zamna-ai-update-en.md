---
title: "Skipping BJJ, Building AI — ZAMNA Hawaii and New Products"
date: 2026-03-19
tags: [bjj, ai, SOLUNA, Koe, zamna, enabler]
description: "BJJ update, why I'm obsessed with AI development, ZAMNA Hawaii in September, and the products that came from it."
---

# Skipping BJJ, Building AI

Let me be honest. I've been skipping jiu-jitsu lately.

Training once a week at best. Competitions haven't been great. The reason is clear: **I'm obsessed with AI development.**

I'm juggling multiple projects simultaneously, and context-switching between them eats up entire days. I still walk the dog, stretch, eat proper meals, and grab lunch with friends — but my mindshare is almost entirely consumed by AI development.

## BJJ Update

Once a week. My body feels heavy. I keep noticing moments in sparring where I think "I used to do this move so much smoother."

Competitions have been mixed. But I'm not quitting, and I'll be competing again soon. BJJ is the only time my brain stops thinking about API endpoints.

---

## Why AI Development Is So Addicting

The past few months have been the most fun I've had coding in my life.

I've been building with Claude Code (an AI that runs in the terminal), and things that used to take 3 months now take 3 days. The code quality is high. It writes tests. It deploys. The result: way too many products.

### Products (Live or Launching Soon)

**[Chatweb.ai](https://chatweb.ai) / [teai.io](https://teai.io)** — AI chat platform. Rust/axum on AWS Lambda. Auto-switches between Claude, Gemini, Nemotron, Qwen for optimal responses.

**[Elio](https://elio.love)** — iOS AI assistant with P2P distributed inference. Devices share compute resources. On the App Store.

**[StayFlow](https://stayflowapp.com)** — Property management for vacation rentals. 500+ facilities. Beds24 integration, automated messaging.

**[JiuFlow](https://jiuflow.art)** — BJJ technique database. 100+ top athletes profiled. (I build BJJ tools while skipping BJJ.)

**[Pasha](https://pasha.run)** — Snap a receipt, auto-bookkeep. OCR + AI classification + blockchain anchoring. [TestFlight](https://testflight.apple.com/join/CTmyqV6H).

**[Koe](https://koe.live)** — Voice input tool + distributed audio. Local speech recognition with whisper.cpp.

And the biggest project...

---

## ZAMNA Hawaii — September 4th

**We're putting on ZAMNA Hawaii on September 4, 2026.** A music + technology festival.

This event spawned several projects.

Details: **[solun.art](https://solun.art)**

---

## Soluna — Turning a Crowd into an Instrument

The system I'm building for ZAMNA.

### Concept

**Sync every device in the venue into one giant speaker.**

1. Attendees join a channel from their phone or device
2. All devices play audio at the exact same NTP-synchronized moment
3. **One device is a memory. A hundred are an orchestra. A thousand are a wave.**

### How It Works

#### Gossip P2P — No Server, Infinite Scale

Instead of routing through a central server, each device forwards packets to its 2-3 nearest peers. They forward to theirs. Like a virus, it reaches everyone in under 10 hops. 1,000 devices? No problem. 10,000? Same.

#### Acoustic Ranging — Measuring Distance with Sound

Devices emit 19.5kHz ultrasonic chirps. Other devices detect them via microphone. Speed of sound is 343 m/s, so the NTP timestamp difference gives **physical distance to ±10cm accuracy.**

This enables:
- **Smart routing**: Forward to physically closest devices, not just lowest WiFi RTT
- **Spatial audio**: Farther devices automatically quieter
- **Distance map**: 2D visualization of device positions on the dashboard

#### Audio DSP Pipeline (All Zero-Allocation)

```
Mic → Highpass Filter → Echo Cancel → Noise Gate
→ AGC → Limiter → Volume → ADPCM Compress → Send
```

All integer math, no heap allocation. Runs with headroom on ESP32 at 240MHz.

#### NTP Sync Playback

Every packet carries a "play at NTP time XX:XX:XX.XXX" timestamp. Receivers buffer (auto-adjusted 20-300ms) and play at exactly the right moment. Whether a packet arrives in 5ms or 50ms, **everyone plays at the same wall-clock time.**

### Dashboard

**[koe.live/dashboard.html](https://koe.live/dashboard.html)** — Real-time visualization of all devices:
- Device list, channels, peers, battery, latency
- Network topology graph
- Acoustic distance map
- **One-click OTA firmware deploy to all devices**

### Multicast OTA — Update 10,000 Devices in 5 Minutes

The same UDP multicast that delivers audio also delivers firmware updates. Upload a binary to the dashboard, it broadcasts as 1KB chunks in a carousel loop. Each ESP32 collects chunks in the background, fills gaps on the next loop, and auto-reboots when complete.

**Updating 1 device or 10,000 devices uses exactly the same bandwidth.**

### Every Platform, Same Protocol

| Platform | Tech | Link |
|---|---|---|
| **ESP32** | Rust (esp-idf), 3,500 lines | [koe.live](https://koe.live) |
| **Web** | WebSocket + WebAudio | [koe.live](https://koe.live) |
| **iPhone** | Swift, Network.framework | Koe-iOS |
| **Mac** | Swift (shared with iOS) | Koe-macOS |
| **Windows** | Rust, cpal | Koe-Windows |
| **Android** | Kotlin, AudioRecord | Soluna Android |

All using the same 19-byte header + IMA-ADPCM + FNV-1a + UDP 239.42.42.1:4242.

---

## What My Days Look Like

I juggle multiple projects, and the context switches eat time. In between, I walk the dog, stretch, eat, have lunch with friends. But honestly, my brain is in AI-dev mode 24/7. Even walking, I'm thinking "if I stream that API response instead of buffering, latency drops by 200ms..."

Claude Code makes it worse. "Wouldn't it be cool if..." → working prototype in an hour. That dopamine hit is addictive.

### What's Next

- **April**: Prototype PCB order (JLCPCB), BJJ competition
- **May-June**: Soluna beta test (small event in Tokyo)
- **July-Aug**: ZAMNA prep, device manufacturing
- **September 4**: **ZAMNA Hawaii**

### Links

| Product | URL |
|---|---|
| Soluna Player | [koe.live](https://koe.live) |
| Dashboard | [koe.live/dashboard.html](https://koe.live/dashboard.html) |
| ZAMNA Hawaii | [solun.art](https://solun.art) |
| Chatweb.ai | [chatweb.ai](https://chatweb.ai) |
| StayFlow | [stayflowapp.com](https://stayflowapp.com) |
| JiuFlow | [jiuflow.art](https://jiuflow.art) |
| Pasha | [pasha.run](https://pasha.run) |
| Elio | [elio.love](https://elio.love) |

To my BJJ training partners: sorry for being MIA. I'll be back on the mats soon. Probably next month.
