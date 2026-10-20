---
title: "未来の自分へ声の手紙を送れる「タイムカプセル」をCloudflare Workers + R2で作った"
date: 2026-10-20
tags: [tech, cloudflare, workers, r2, koe]
description: "KOEに追加したタイムカプセル機能の実装記録。ブラウザ録音、R2保存、Cron Triggerでの定期配信、そしてwrangler versions deployではCron Triggerが反映されない罠について。"
---

# 未来の自分へ声の手紙を送れる「タイムカプセル」を作った

自分の声を録音して、未来の自分に届ける「タイムカプセル」機能を、KOE（koe.live）に追加しました。

- 録音はブラウザのMediaRecorderで完結（最大30秒）
- 届け日は1日後〜10年後まで自由に指定
- 指定日のJST 9:00にメールで通知
- 音声はCloudflare R2に保存、配信はCloudflare WorkersのCron Trigger

## 構成

```
ブラウザ(録音) → Cloudflare Workers(保存) → R2(音声+メタデータ)
                                          ↓
                                    Cron Trigger(毎日UTC 0:00)
                                          ↓
                                    Resend(メール送信)
```

## 詰まったところ: `wrangler versions deploy` ではCron Triggerが反映されない

KOEのデプロイは `wrangler versions upload` → preview smoke → `wrangler versions deploy` という安全な昇格フローを使っています。

しかし、`versions deploy` では **Cron Triggerが反映されません**。デプロイ後にCloudflare APIで確認すると `schedules: []` のまま。

解決策は `wrangler triggers deploy` を明示的に実行すること。

```bash
npx wrangler versions upload --preview-alias preview
# preview smoke...
npx wrangler versions deploy <version-id>@100 --yes
npx wrangler triggers deploy --yes  # ← これが必要
```

`triggers deploy` は `wrangler.toml` の `[triggers] crons` を読んで、Workerにスケジュールを登録します。`versions deploy` とは別の操作です。

## 録音の実装

ブラウザのMediaRecorderでWAVを録音し、base64でPOSTします。

```javascript
const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
const rec = new MediaRecorder(stream);
rec.start(100);
// 30秒で自動停止
setTimeout(() => rec.stop(), 30000);
rec.onstop = async () => {
  const blob = new Blob(chunks, { type: "audio/wav" });
  const fr = new FileReader();
  fr.onload = async () => {
    await fetch("/api/me/capsule", {
      method: "POST",
      body: JSON.stringify({
        audio_b64: fr.result,
        deliver_at: deliverAt,
        note: note,
      }),
    });
  };
  fr.readAsDataURL(blob);
};
```

## 配信の実装

Cron Triggerで毎日UTC 0:00（JST 9:00）に起動し、R2の `capsule/` プレフィックスをスキャン。

```javascript
async function checkCapsules(env) {
  const now = Date.now();
  let cursor, sent = 0;
  do {
    const l = await env.BUCKET.list({ prefix: "capsule/", limit: 200, cursor });
    for (const o of l.objects) {
      if (!o.key.endsWith(".json")) continue;
      const c = JSON.parse(await (await env.BUCKET.get(o.key)).text());
      if (c.delivered || c.deliver_at > now) continue;
      await sendCapsuleEmail(env, c.email, c.note, audioUrl, c.deliver_at);
      c.delivered = true;
      await env.BUCKET.put(o.key, JSON.stringify(c));
      if (++sent >= 10) return; // 1回のcronで最大10件
    }
    cursor = l.truncated ? l.cursor : undefined;
  } while (cursor);
}
```

## セキュリティ

- カプセルは本人のみアクセス可能（ログイン + 声のclaimが必要）
- 音声配信は `Cache-Control: private, no-store`
- 管理APIはトークン認証（URLクエリではなくAuthorizationヘッダー）

## まとめ

- Cloudflare Workers + R2で、サーバーレスなタイムカプセルが作れた
- `wrangler versions deploy` ではCron Triggerが反映されないので、`triggers deploy` を忘れずに
- ブラウザのMediaRecorderで、プラグインなしで録音できる

KOEでは他にも「声のタイムライン」（過去の声を時系列で聴ける）も作っています。声を残す・届ける・振り返る、という体験をこれからも磨いていきます。

## リンク

- [KOE — 声で、なんでも。](https://koe.live)
- [未来への声の手紙](https://koe.live/me/capsule)（ログインが必要です）
