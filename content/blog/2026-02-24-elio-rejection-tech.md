---
title: "App Store審査対応の技術詳細 — URLSession timeout, InfoPlist.strings, China DST"
date: "2026-02-24"
description: "Elio ChatのApp Store 3回リジェクト→承認までに行った技術的修正の全記録"
tags: ["Elio", "iOS", "Swift", "tech"]
---

> **経験談・物語版** → [App Store審査リジェクト4連発を乗り越えた話](/blog/2026-02-24-elio-rejection-marathon)

## 審査リジェクトの時系列

Elio Chatは3回リジェクトされた。各リジェクトの理由と対応を技術的に記録する。同じ轍を踏むiOS開発者の参考になれば幸いだ。

| 回 | 日付 | 理由コード | 要約 |
|----|------|-----------|------|
| 1 | 2/15 | 2.1 Performance | ネットワーク未接続時にクラッシュ |
| 2 | 2/19 | 2.3.1 Performance | P2P接続タイムアウトで無限ローディング |
| 3 | 2/22 | 5.2.1 Legal | GPT参照が残っている、中国DST未対応 |

## リジェクト1: URLSession timeout設計の根本修正

最初のリジェクトは、機内モード状態でアプリを起動すると P2P推論のリクエストでクラッシュするというものだった。原因は `URLSession` のデフォルト設定にあった。

修正前:

```swift
// 修正前: デフォルトのURLSessionConfiguration
let session = URLSession.shared
let task = session.dataTask(with: request) { data, response, error in
    // error が nil でない場合の処理が不十分
    guard let data = data else { return } // ← ここで暗黙にdropしていた
}
```

修正後は3つの層で防御する設計に変更した。

```swift
// ElioNetworkManager.swift — 修正後

final class ElioNetworkManager: NSObject, URLSessionTaskDelegate {
    static let shared = ElioNetworkManager()

    private lazy var session: URLSession = {
        let config = URLSessionConfiguration.default

        // Layer 1: タイムアウト設定
        // timeoutIntervalForRequest: 個々のリクエストのタイムアウト
        config.timeoutIntervalForRequest = 30
        // timeoutIntervalForResource: リソース全体のタイムアウト
        // 600s → 3600s に変更。P2P推論は大規模モデルだと10分以上かかる
        config.timeoutIntervalForResource = 3600
        // Layer 2: 接続待ちを有効化
        config.waitsForConnectivity = true

        return URLSession(
            configuration: config,
            delegate: self,
            delegateQueue: .main
        )
    }()

    // Layer 3: 接続待ち状態のデリゲート通知
    func urlSession(
        _ session: URLSession,
        taskIsWaitingForConnectivity task: URLSessionTask
    ) {
        // UIに「接続待ち」状態を通知
        NotificationCenter.default.post(
            name: .elioWaitingForConnectivity,
            object: nil,
            userInfo: ["taskId": task.taskIdentifier]
        )
    }
}
```

`waitsForConnectivity = true` と `taskIsWaitingForConnectivity` デリゲートの組み合わせがカギだ。これにより、オフライン時にエラーを投げる代わりに接続復帰を待ち、その間ユーザーには「接続を待っています...」のUIを表示する。

`timeoutIntervalForResource` を600秒から3600秒に延ばした理由は、P2P分散推論でリモートノードの応答が遅い場合があるため。ただし通常リクエストには30秒の `timeoutIntervalForRequest` が先に効くので、一般的なAPIコールが遅くなることはない。

## リジェクト2: P2P接続のグレースフルデグラデーション

2回目はP2Pノードが見つからない場合に無限ローディングになる問題。これはタイムアウト自体は設定されていたが、UIのステートマシンが不完全だった。

```swift
// InferenceCoordinator.swift

enum InferenceState {
    case idle
    case searchingPeers        // P2Pノード探索中 (最大15秒)
    case connectingToPeer      // ノード接続中
    case waitingForInference   // 推論待ち
    case streaming(String)     // ストリーミング受信中
    case fallbackToLocal       // ローカルモデルにフォールバック ← 追加
    case completed(String)
    case error(ElioError)
}

// 15秒でP2Pノードが見つからなければローカル推論にフォールバック
func startInference(prompt: String) async {
    state = .searchingPeers

    let peerResult = await withTaskGroup(of: PeerNode?.self) { group in
        group.addTask { await self.p2pManager.discoverPeers(timeout: 15) }
        group.addTask {
            try? await Task.sleep(for: .seconds(15))
            return nil  // タイムアウトセンチネル
        }
        // 最初に返った結果を採用
        for await result in group {
            group.cancelAll()
            return result
        }
        return nil
    }

    if let peer = peerResult {
        state = .connectingToPeer
        await performDistributedInference(peer: peer, prompt: prompt)
    } else {
        state = .fallbackToLocal
        await performLocalInference(prompt: prompt)
    }
}
```

`withTaskGroup` による構造化並行処理で、P2P探索とタイムアウトを競合させる。これによりP2Pが使えない環境でも必ず15秒以内にローカル推論に遷移する。

## リジェクト3: GPT参照削除78箇所と中国DST対応

3回目は法務関連。アプリ内に「GPT」「OpenAI」の参照が78箇所残っていた。これはP2P推論のプロトコル名やログ出力に含まれていたもので、UIには表示されないが、バイナリスキャンで検出されたらしい。

対応はシンプルだが地味な作業だった。

```swift
// 修正前
let modelName = "gpt-4o-mini"
Logger.info("Querying OpenAI API...")

// 修正後
let modelName = "elio-inference-v1"
Logger.info("Querying inference endpoint...")
```

78箇所をgrepで洗い出し、一括置換した。`rg -i "gpt|openai" --type swift` で漏れがないことを確認。

中国DST (Data Security Technology) 対応は、`NSPrivacyAccessedAPICategoryDiskSpace` 等のPrivacy Manifest追加だった。Xcode 15.3以降で必須になったPrivacy Manifestに、使用しているAPIカテゴリと使用理由を宣言する。

## 12言語 InfoPlist.strings

App Store表示名のローカライズも審査指摘に含まれていた。`CFBundleDisplayName` が英語のみだった。

```
// ja.lproj/InfoPlist.strings
CFBundleDisplayName = "Elio";
CFBundleName = "Elio";
NSMicrophoneUsageDescription = "音声入力に使用します";
NSCameraUsageDescription = "プロフィール写真の撮影に使用します";

// zh-Hans.lproj/InfoPlist.strings
CFBundleDisplayName = "Elio";
CFBundleName = "Elio";
NSMicrophoneUsageDescription = "用于语音输入";
NSCameraUsageDescription = "用于拍摄个人资料照片";
```

12言語分のファイルを作成。アプリ名「Elio」自体は固有名詞なので全言語共通だが、permission descriptionは各言語に翻訳が必要だ。

## 教訓

1. **`waitsForConnectivity` は必須**: ネットワーク依存アプリでこれを設定しないのは審査落ちの原因になる
2. **サードパーティ名のバイナリスキャン**: UIに表示されなくても文字列リテラルは検出される。CIで `strings` コマンドによる自動チェックを入れるべき
3. **Privacy Manifest**: 使用APIの宣言漏れは自動リジェクト。`xcodebuild -showBuildSettings` でリンクしているフレームワークを洗い出し、各フレームワークの必要APIカテゴリを事前に確認する

3回のリジェクトで計9日を失った。これらのチェックをCIに組み込んでおけば、初回で通過できたはずだ。
