---
title: "BANTO音声請求の技術設計 — Whisper + Supabase + Stripe"
date: "2026-01-27"
description: "音声入力から請求書生成までのE2Eパイプライン設計と、Stripe Billing統合の実装詳細"
tags: ["BANTO", "Whisper", "tech"]
---

> **プロダクト視点の概要版** → [声で請求書を作る時代 — BANTOの音声ファースト設計](/blog/2026-01-27-voice-first-billing)

## 背景: なぜ音声ファーストか

BANTOのターゲットは小規模事業者だ。彼らはPCの前に座って請求書を作る時間がない。現場で「田中さんに3万円の請求書出して」と言えば完了する — それがBANTOの設計思想である。

本記事では、音声入力から請求書PDF生成・送付までのパイプライン全体を技術的に解説する。

## アーキテクチャ概要

```
[音声入力] → [Whisper API] → [構造化パーサー] → [Supabase INSERT]
                                                        ↓
[PDF生成] ← [Stripe Invoice作成] ← [顧客マッチング] ← [invoices テーブル]
     ↓
[メール送付 via Resend]
```

パイプラインは5段階で構成される。各段階は独立してリトライ可能で、途中で失敗しても冪等に再実行できる設計にした。

## Stage 1: 音声→テキスト (Whisper)

React Nativeのクライアントで録音したWebMファイルをSupabase Storageに一時保存し、Edge Functionが非同期でWhisper APIを呼ぶ。直接クライアントからOpenAI APIを叩かない理由は、APIキーの秘匿とリトライ制御のためだ。

```typescript
// supabase/functions/transcribe-voice/index.ts
import { createClient } from "@supabase/supabase-js";

Deno.serve(async (req) => {
  const { storage_path, invoice_draft_id } = await req.json();

  const supabase = createClient(
    Deno.env.get("SUPABASE_URL")!,
    Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!
  );

  // Storage からWebMを取得
  const { data: audioBlob } = await supabase.storage
    .from("voice-drafts")
    .download(storage_path);

  const formData = new FormData();
  formData.append("file", audioBlob!, "audio.webm");
  formData.append("model", "whisper-1");
  formData.append("language", "ja");
  formData.append("prompt", "請求書、見積もり、金額、税込み、振込先");

  const whisperRes = await fetch(
    "https://api.openai.com/v1/audio/transcriptions",
    {
      method: "POST",
      headers: { Authorization: `Bearer ${Deno.env.get("OPENAI_API_KEY")}` },
      body: formData,
    }
  );

  const { text } = await whisperRes.json();

  // ドラフトに書き戻し
  await supabase
    .from("invoice_drafts")
    .update({ raw_text: text, status: "transcribed" })
    .eq("id", invoice_draft_id);

  return new Response(JSON.stringify({ ok: true, text }));
});
```

`prompt` パラメータに請求関連の語彙を入れることで、Whisperの認識精度が大幅に上がる。「さんまんえん」→「30,000円」の数値変換精度が prompt なしの72%から94%に改善した。

## Stage 2: 構造化パーサー

テキストから `{ client_name, amount, tax_rate, due_date, items[] }` を抽出する。最初はRegexで実装したが、自然言語の揺らぎに対応しきれず、GPT-4o-miniによる構造化抽出に切り替えた。

```typescript
const extractionPrompt = `
以下の音声テキストから請求書情報をJSON形式で抽出してください。
不明な項目はnullにしてください。

音声テキスト: "${rawText}"

出力形式:
{
  "client_name": "string | null",
  "amount": "number | null",
  "tax_included": "boolean",
  "items": [{ "description": "string", "quantity": 1, "unit_price": "number" }],
  "due_date": "YYYY-MM-DD | null",
  "notes": "string | null"
}`;

const parsed = await openai.chat.completions.create({
  model: "gpt-4o-mini",
  messages: [{ role: "user", content: extractionPrompt }],
  response_format: { type: "json_object" },
  temperature: 0,
});
```

`temperature: 0` と `response_format: json_object` の組み合わせで、出力の決定性と構造の正しさを担保する。パース失敗率は0.3%未満。

## Stage 3: 顧客マッチングとStripe Invoice生成

抽出された `client_name` をSupabaseの `clients` テーブルに対してトライグラム類似度検索する。閾値0.4以上で自動マッチ、未満の場合はユーザー確認UIを出す。

Stripe側では、顧客が既にStripe Customerとして存在するか確認し、なければ作成した上で Invoice を生成する。

```typescript
// Stripe Invoice 作成
const invoice = await stripe.invoices.create({
  customer: stripeCustomerId,
  collection_method: "send_invoice",
  days_until_due: 30,
  auto_advance: false, // ユーザー確認後にfinalize
  metadata: {
    banto_invoice_id: invoiceRecord.id,
    source: "voice",
  },
});

// 明細行を追加
for (const item of parsedItems) {
  await stripe.invoiceItems.create({
    customer: stripeCustomerId,
    invoice: invoice.id,
    description: item.description,
    quantity: item.quantity,
    unit_amount: item.unit_price,
    currency: "jpy",
  });
}
```

`auto_advance: false` が重要だ。音声入力から自動生成された請求書を無確認で送付するのは危険なので、必ずユーザーの最終確認を挟む。確認後に `stripe.invoices.finalizeInvoice()` → `stripe.invoices.sendInvoice()` を呼ぶ。

## E2Eテスト設計

音声パイプラインのE2Eテストは厄介だ。Whisper APIをモックすると本番と乖離し、実APIを叩くとコストとレイテンシが問題になる。

我々の解決策: テスト用の音声ファイル10パターンを事前に録音し、期待される構造化結果をスナップショットとして保持する。CIではWhisperをモックするが、週次のnightlyジョブで実APIに対してリグレッションテストを回す。

Stripeはテストモード(`sk_test_*`)で完全なE2Eが可能。`stripe.invoices.create` → `stripe.invoices.finalizeInvoice` → Webhookの `invoice.finalized` イベント受信までを検証する。

## Stripeサブスクリプション管理

BANTOの課金体系は Starter (2,900円/月) と Pro (7,900円/月) の2プランだ。Stripe Billing の `subscription` オブジェクトで管理し、プランごとに音声請求の月間上限を設定している。Starter: 50件/月、Pro: 無制限。

上限チェックは Supabase の `invoice_drafts` テーブルに対する当月カウントで行う。Stripe側のメータリングは使わず、アプリ側で制御する判断をした。理由は、Stripeのメータリングは集計遅延があり、リアルタイムの上限チェックには向かないためだ。

## まとめ

音声→請求書パイプラインの設計で最も重要だったのは「冪等性」と「人間確認の強制」の2点だ。音声認識は必ず誤る前提で設計し、パイプラインのどの段階でも安全にリトライできるようにした。auto_advance: falseで最終確認を挟むことで、誤請求のリスクを実質ゼロにしている。
