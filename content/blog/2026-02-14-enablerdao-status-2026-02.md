---
title: "EnablerDAO - 2026年2月プロジェクトレポート"
date: "2026-02-14"
author: "Yuki Hamada"
tags: ["EnablerDAO", "プロジェクト報告", "Web3", "AI", "SaaS"]
description: "EnablerDAOの12プロジェクトの最新状況、技術スタック、収益モデル、今後の展望をまとめました。"
image: "/blog/enablerdao-status-2026-02.png"
---

# EnablerDAO 2026年2月プロジェクトレポート

こんにちは、濱田優貴です。EnablerDAOで開発中の12プロジェクトについて、最新状況をレポートします。

## TL;DR

- 📊 **12プロジェクト稼働中**（月間6,000+訪問）
- 💰 **現在MRR**: ¥280,000/月
- 🎯 **12ヶ月目標**: ¥3,000,000/月
- 🚀 **最新**: enablerdao.com Cloudflare Pages移行完了

---

## プロジェクト概要

### 🤖 AI & テクノロジー

#### 1. Chatweb.ai - AI駆動のWeb自動化
音声やテキストで指示するだけで、AIがブラウザを操作して作業を自動化。LINE・Telegramでも使えます。

- **技術**: Rust Lambda, Next.js
- **デプロイ**: AWS Lambda (ap-northeast-1)
- **トラフィック**: 289 visits/月
- **最新機能**: Agentic Mode（マルチイテレーションツール実行）
- **料金**: Free / $9/月 / $29/月

#### 2. Wisbee（Chatweb.aiと統合）
プライバシー重視のAIアシスタント。Chatweb.aiと統合され、より強力な機能を提供しています。

- **トラフィック**: 613 visits/月
- **統合完了**: 2026年2月

#### 3. Elio Chat - iPhoneで完全オフラインAI
通信不要で動作するAIチャットアプリ。データは端末内のみで処理され、プライバシーを完全保護。

- **プラットフォーム**: iOS (App Store)
- **トラフィック**: 506 visits/月
- **次のステップ**: Product Hunt ローンチ準備中

#### 4. News.xyz - AIニュース配信
AIが複数テーマから自動でニュースを収集・配信。好みの記事を効率的に読めます。

- **トラフィック**: 506 visits/月
- **次のステップ**: Product Hunt ローンチ準備中

---

### 💼 ビジネスツール

#### 5. StayFlow - 民泊・宿泊施設運営管理（最大トラフィック）
予約・清掃・チェックインを一元管理。Airbnb等の連携で運営効率を大幅向上。

- **トラフィック**: 1,840 visits/月 ⭐️ **最多**
- **技術**: Next.js, Supabase
- **料金**: ¥5,000/月〜

#### 6-9. その他SaaS
- **BANTO**: 建設業向け請求書管理（186 visits/月）
- **Totonos**: 企業財務自動化（103 visits/月）
- **VOLT**: ライブオークション（205 visits/月）
- **Enabler**: ライフスタイルサービス（107 visits/月）

---

### 🔒 セキュリティ

#### 10. Security Scanner - 無料Webセキュリティ診断
URLを入れるだけでWebサイトの安全性をA〜Fで評価。8種類以上のセキュリティヘッダーをチェック。

- **トラフィック**: 113 visits/月
- **料金**: Free / $19/月（Pro）

---

### 🥋 スポーツ & コミュニティ

#### 11. JitsuFlow - ブラジリアン柔術アプリ
練習記録・道場運営を効率化。技術習得の進捗を可視化します。

- **トラフィック**: 1,310 visits/月
- **デプロイ**: Fly.io (nrt)
- **技術**: Next.js, Supabase

---

## 技術スタックと哲学

### なぜこれらの技術を選んだか

#### Edge-First Architecture
Cloudflare Pages, Workers, Fly.ioを活用し、グローバルで低レイテンシな配信を実現。

#### TypeScript + Rust
- **TypeScript**: フロントエンド・バックエンドの型安全性
- **Rust**: Lambda関数の高速実行・低コスト

#### Supabase
オープンソースのFirebase代替。PostgreSQL + リアルタイムサブスクリプション。

---

## 収益モデル

### サブスクリプション（メイン）
- Chatweb.ai Pro: $9-29/月
- Elio Chat Pro: $4.99/月
- StayFlow: ¥5,000-50,000/月
- Security Scanner Pro: $19/月

### トランザクション
- VOLT: 取引手数料10%
- Enabler: マーケットプレイス手数料15%

### 現状と目標

| 期間 | MRR | 有料ユーザー |
|------|-----|--------------|
| **現在** | ¥280,000 | 20 |
| **3ヶ月後** | ¥500,000 | 100 |
| **6ヶ月後** | ¥1,500,000 | 300 |
| **12ヶ月後** | ¥3,000,000 | 1,600 |

---

## 最近の主要更新（2026年2月）

### enablerdao.com リニューアル

1. **Cloudflare Pagesへ移行**
   - Edge Runtime対応
   - グローバル330拠点CDN
   - 無料枠内で運用可能

2. **コンバージョン最適化**
   - Newsletter CTA追加
   - プロダクトカード強化（価格・ユーザー数表示）
   - ヒーローCTA改善（"無料で始める"）
   - トラストバッジ（12製品/6,000+ユーザー）

3. **技術改善**
   - 全APIルート Edge Runtime対応
   - Web Crypto API移行（Node.js依存排除）
   - GitHub Actions CI/CD設定

---

## DAOとしての運営

### EBRトークン
EnablerDAOは**投票トークン（EBR）**で運営されています。

- **総供給**: 1,000,000 EBR
- **獲得方法**: コード貢献、バグ報告、ドキュメント作成
- **用途**: プロジェクトの方向性を決める投票

投資商品ではなく、**ガバナンスのツール**です。

---

## 次のステップ

### 短期（1ヶ月以内）
- [ ] enablerdao.com デプロイ完了
- [ ] totonos.jp 戦略決定
- [ ] news.xyz Product Hunt ローンチ
- [ ] elio.love App Store申請

### 中期（3ヶ月以内）
- [ ] 広告キャンペーン開始（Reddit/Google/Apple）
- [ ] Newsletter 1,000購読者達成
- [ ] 有料ユーザー100名突破

### 長期（12ヶ月以内）
- [ ] MRR ¥3,000,000達成
- [ ] チーム5名雇用
- [ ] DAOコミュニティ1,000名

---

## 参加方法

EnablerDAOは誰でも参加できるオープンな組織です。

1. **まず使ってみる**: [Security Scanner](https://chatnews.tech)等の無料ツールを試す
2. **フィードバックする**: [GitHub](https://github.com/yukihamada)で改善提案
3. **貢献する**: コード、ドキュメント、バグ報告でEBRトークンを獲得
4. **投票する**: プロジェクトの方向性を決める

---

## まとめ

12プロジェクトを並行開発・運営しながら、月間6,000訪問、MRR ¥280,000を達成しました。

次の目標は**12ヶ月でMRR ¥3,000,000**です。

技術選定（Edge-First, TypeScript, Rust）、収益モデル（サブスク+トランザクション）、DAOガバナンス（EBRトークン）の3つの柱で、持続可能な成長を目指します。

興味がある方は、ぜひ[enablerdao.com](https://enablerdao.com)をご覧ください。

---

**濱田優貴** / [yukihamada.jp](https://yukihamada.jp)
EnablerDAO Founder
GitHub: [@yukihamada](https://github.com/yukihamada)
X: [@yukihamada](https://x.com/yukihamada)
