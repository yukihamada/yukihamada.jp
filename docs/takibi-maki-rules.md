# 薪の発行ルール（ともしび焚き火）

決定: 2026-06-05 / 対象: yukihamada.jp `/ともしび`（`src/community.rs`）

## 芯
**薪 = 1つの本物の熱（実作業・実イベント）。** 火の大きさ＝今この瞬間の本物の熱量。
だから「作業した分だけ火が育つ」が成り立つように、発行はレート規律で守る。
1人が連投で火やトレンド（薪の増減メーター）を偽装できてはいけない。

## 発行チャネル（誰/何が薪を出せるか）
| チャネル | 認証 | kind | 制限 |
|---|---|---|---|
| 人（add_log） | Bearer api_token | `log` | R2 + R3 |
| 着火（ignite） | Bearer api_token | `ignite` | R2 |
| webhook（commit/deploy/sale等） | `COMMUNITY_WEBHOOK_SECRET` | 種別マップ | 本物のイベントのみ・1イベント1薪（R4） |
| 記念日 | anniversaries.json | `memorial` | 1日1回（R6） |
| federation relay | peer設定 | `relay` | 再relayしない・dedup（R5） |
| seed | 起動時 | `ignite` | 空のとき1回だけ |

## ルール
- **R1 認証必須**: 書き込みは Bearer api_token のみ（cookieフォールバック無し＝CSRF不可）。誰でもメール認証で取得できるが匿名スパムは不可。
- **R2 レート規律**（env調整可・再デプロイ不要）:
  - 連投の最小間隔 `COMMUNITY_RATE_GAP_SECS`（既定 **90秒**）
  - 1時間あたり `COMMUNITY_RATE_HOURLY`（既定 **20本**）
  - 1日あたり `COMMUNITY_RATE_DAILY`（既定 **60本**）
  - 根拠: 10分TTLの火は10分に1本で維持できる。3分に1本（時20本）が「実作業の鼓動」の自然な上限。超過は429相当で人向けメッセージを返す。
- **R3 空薪・重複の排除**: 2文字未満は発行不可。直近1時間の同一発行者・同一本文（空白正規化）は重複として拒否。
- **R4 webhookは本物のイベントのみ**: secret必須・種別ホワイトリスト・1イベント1薪。自由テキスト投稿は人の手（add_log）に寄せる。
- **R5 relayは二次燃料**: 他コミュニティの火は取り込むが再relayしない（ループ防止）。`kind=relay` のまま＝自家発電の熱と区別できる。
- **R6 システム薪**: 記念日/seedは author を人と混ぜない（`memorial`/`ignite`）。
- **R7 不可逆・追記のみ**: 薪は消せない。10分で建物へアーカイブ。削除でトレンドを操作できない。
- **R8 ASH/OKIとの境界**: 薪は「熱の信号」であって通貨ではない。円・購入・譲渡なし。価値レイヤー（感謝→灰ASH/熾火OKI）は `atsm-token`（kenny合議）側で、**薪をくべてもトークンは発行しない**。混ぜない。

## 調整したくなったら
`COMMUNITY_RATE_GAP_SECS` / `COMMUNITY_RATE_HOURLY` / `COMMUNITY_RATE_DAILY` を Fly secrets で変えるだけ（コード変更不要）。
