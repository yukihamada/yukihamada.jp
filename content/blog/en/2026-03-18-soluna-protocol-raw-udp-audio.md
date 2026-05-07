---
title: "SOLUNA — Building a Protocol Where 1,000 People Feel the Same Beat at the Same Moment"
date: 2026-03-18
tags: [SOLUNA, Protocol, Audio, Rust]
description: "1,000 speakers playing simultaneously at a festival. Impossible with existing tech. We're doing it with UDP and a 19-byte header."
---

# 1,000 People Feel the Same Beat at the Same Moment

At a festival, the further you are from the stage, the more the sound delays. Sound only travels at 343 m/s. At 50 meters, that's 145ms of lag.

But what if every device in the audience played the same sound at **the exact same moment**?

The crowd becomes the instrument. The entire venue becomes the speaker.

That's what SOLUNA is building.

## Why Existing Tech Can't Do This

Spotify, AirPlay, Bluetooth — all have 150ms+ latency. Humans perceive "simultaneous" within 20ms. Off by an order of magnitude.

The reason is simple: they compress audio first, then send it over TCP. Compression takes time. TCP waits for lost packets to be retransmitted. Too polite.

## Send Raw Audio First

SOLUNA does the opposite.

**Send raw audio over UDP first. Send compressed data afterward to save bandwidth.**

UDP doesn't care about dropped packets. Humans don't notice 0.1 seconds of missing audio. And raw audio has zero encoding time — data goes straight from the microphone to the wire.

We also use UDP multicast. One packet reaches all 1,000 devices on the same WiFi. No need to send it 1,000 times.

The header is just 19 bytes. Less than 1/100th of HTTP. Everything else is audio.

## A 26mm Device

The first product is "COIN." A 26mm round PCB with an ESP32-S3, microphone, and speaker. BOM: $24.

Hand them out at the festival entrance. Every COIN joins the same channel and plays the same beat at the same moment. No setup. No pairing. Just power on.

Total equipment cost for a 10,000-person festival: ~$300K. One-third of a conventional L-Acoustics system.

## Rust From Firmware to Protocol

ESP32 firmware, Pi5 server, CLI tools — all Rust. Same code runs on every platform.

The spec will be open-sourced. Interested in a festival pilot? Reach out at [mail@yukihamada.jp](mailto:mail@yukihamada.jp).
