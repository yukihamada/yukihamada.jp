---
title: A Day of Jiu-Jitsu and Coding
date: 2026-02-13
tags: [bjj, coding, enablerdao]
description: Morning spent on service checks and deploys, afternoon at jiu-jitsu practice. On the parallels between engineering and jiu-jitsu.
---

## Morning Work

This morning I ran through operational checks on all EnablerDAO services.

- **All 55 tests passing** -- EnablerDAO, DojoC, 12 external services, and security headers all green
- **DojoC Next.js 16 upgrade** -- Security vulnerabilities reduced from 11 to 0
- **EnablerDAO CLI tool completed** -- Install with `curl -fsSL https://enablerdao.com/install.sh | bash`
- **Blog feature launched** -- Tech blog now live at enablerdao.com/blog
- **Migrated this site (yukihamada.jp) to Rust/Axum** -- Faster performance and blog support added

## Jiu-Jitsu

In the afternoon I headed to jiu-jitsu practice. There may be another session at 17:00.

Jiu-jitsu and programming have a lot in common:

- **Position** = **Architecture** -- Good position leads to good attacks. Good architecture leads to good code.
- **Escapes** = **Debugging** -- The art of getting out of a bad situation. Identify the cause and follow the right steps to escape.
- **Sparring** = **Testing** -- You won't know until you actually run it. Theory alone is never enough.
- **Belt** = **Experience** -- There are no shortcuts. Consistent practice builds real skill.

## Afternoon Plans

After jiu-jitsu, I plan to wrap up the remaining tasks:

- Finish deploying yukihamada.jp on Fly.io
- Switch over Cloudflare DNS
- Write additional blog posts
