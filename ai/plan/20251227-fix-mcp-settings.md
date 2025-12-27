---
mission_id: digrag-search-improve
title: "digrag 検索機能改善 - MCP設定・BM25・ハイブリッド検索の修正"
status: planning
progress: 0
phase: planning
tdd_mode: true
blockers: 0
created_at: "2025-12-27"
updated_at: "2025-12-27"
---

# Commander's Intent

## Purpose
ユーザーが ~/repos/changelog で digrag MCP を使用して「vimconf2025」を検索できるようにする。現在、MCP設定の問題、BM25検索の制限、ハイブリッド検索のセマンティック機能が動作していないため、これらを段階的に修正する必要がある。

## End State
- MCP設定が正しく機能し、digrag がツールリストに表示される
- 「vimconf2025」をBM25検索で見つけられる
- ハイブリッド検索でセマンティック検索結果が取得できる
- 既存テストが全て通過する
- cargo clippy の警告がない

## Key Tasks
- MCP設定のチルダ展開問題を絶対パスで解決
- BM25インデックスにタイトルを含める
- 英語トークン抽出で数字を含める（vimconf2025対応）
- CamelCase分割を実装（VimConf分割対応）
- ハイブリッド検索に embedding client を設定
- インデックス再構築と検索検証

## Constraints
- 既存のテストを壊さない
- 後方互換性を維持（既存インデックスの読み込みは継続サポート）

## Restraints
- cargo test が全て通過すること
- cargo clippy で警告がないこと

---

# Context

## 概要
digrag MCP を使用して changelog ドキュメント（647ドキュメント）から「vimconf2025」を検索できるようにする実装。現在3つの問題が発生している：

1. **MCP設定問題**: チルダ展開されず digrag が認識されない
2. **BM25検索問題**: タイトルを検索対象に含まず、数字付きキーワードを除外している
3. **ハイブリッド検索問題**: embedding client が設定されておらず、セマンティック検索が機能していない

この3つの問題を段階的に解決することで、ユーザーが効率的に検索できるようにする。

## 必須のルール
- 必ず `CLAUDE.md` を参照し、ルールを守ること
- **TDD（テスト駆動開発）を厳守すること**
  - 各プロセスは必ずテストファーストで開始する（Red → Green → Refactor）
  - 実装コードを書く前に、失敗するテストを先に作成する
  - テストが通過するまで修正とテスト実行を繰り返す
  - プロセス完了の条件：該当するすべてのテスト、フォーマッタ、Linterが通過していること

## 開発のゴール
- digrag の検索機能を完全に動作させる
- BM25検索でキーワード「vimconf2025」を見つけられる
- ハイブリッド検索でセマンティック検索結果が取得できる
- ユーザーが MCP から digrag を利用できる環境を整える

---

# References

| @ref | @target | @test |
|------|---------|-------|
| ~/.mcp.json | ~/repos/changelog/.mcp.json | N/A |
| BM25実装 | /home/takets/repos/digrag/src/index/bm25.rs | /home/takets/repos/digrag/src/index/bm25.rs (tests) |
| トークナイザー | /home/takets/repos/digrag/src/tokenizer/japanese.rs | /home/takets/repos/digrag/src/tokenizer/japanese.rs (tests) |
| CLI/MCP | /home/takets/repos/digrag/src/main.rs | /home/takets/repos/digrag/tests/ |
| 検索ロジック | /home/takets/repos/digrag/src/search/searcher.rs | /home/takets/repos/digrag/src/search/searcher.rs (tests) |
| インデックス | ~/repos/changelog/.rag/ | N/A |

---

# Progress Map

| Process | Status | Progress | Phase | Notes |
|---------|--------|----------|-------|-------|
| Process 1: MCP設定の修正 | planning | ▯▯▯▯▯ 0% | Red | チルダ展開問題の解決 |
| Process 2: タイトルをBM25対象に含める | planning | ▯▯▯▯▯ 0% | Red | bm25.rs:80 の修正 |
| Process 3: 英語トークンに数字を含める | planning | ▯▯▯▯▯ 0% | Red | 正規表現の変更 |
| Process 4: CamelCase分割 | planning | ▯▯▯▯▯ 0% | Red | split_camel_case 関数追加 |
| Process 5: ハイブリッド検索でembedding client使用 | planning | ▯▯▯▯▯ 0% | Red | main.rs/searcher.rs の修正 |
| Process 10: テスト統合 | planning | ▯▯▯▯▯ 0% | Red | cargo test 通過確認 |
| Process 50: インデックス再構築と検証 | planning | ▯▯▯▯▯ 0% | planning | 実環境での検証 |
| **Overall** | **planning** | **▯▯▯▯▯ 0%** | **planning** | **Blockers: 0** |

---

# Processes

## Process 1: MCP設定の修正

### Red Phase: テスト作成と失敗確認
- [ ] MCP設定が正しく読み込まれることを確認するテストを作成
  - ~/.mcp.json の digrag コマンドパスが絶対パスで設定されていることを検証
  - MCP起動時に digrag が正常に呼び出されることを確認
- [ ] テストを実行して失敗することを確認（現在チルダ展開されていない状態を検証）

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] ~/repos/changelog/.mcp.json 内の digrag コマンドパスを `~/.local/bin/digrag` から `/home/takets/.local/bin/digrag` に変更
  - Optional: 他の MCP設定ファイルが存在する場合も同様に修正
- [ ] MCP設定が正しく読み込まれることをテストで確認
- [ ] digrag がツールリストに表示されることを手動確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] 他の潜在的なパス設定を確認
- [ ] テストが継続して成功することを確認

✅ **Phase Complete**

---

## Process 2: タイトルをBM25対象に含める

### Red Phase: テスト作成と失敗確認
- [ ] タイトルのみに含まれるキーワードで検索するテストを作成
  - 「VimConf」をドキュメントのタイトルに含むテストドキュメントを作成
  - そのドキュメントを検索対象に含める
  - 「VimConf」で検索して見つかることを期待するテストを記述
- [ ] テストを実行して失敗することを確認（現状、テキスト本体のみを対象としているため）

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] /home/takets/repos/digrag/src/index/bm25.rs の bm25_index_doc 関数（行80付近）を修正
  - `doc.text` のみでなく、`format!("{} {}", doc.title(), doc.text)` でタイトルも結合してトークン化
  - Optional: タイトルに高い重みを付けることを検討
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] コードの可読性を改善（タイトルとテキストの結合方法を最適化）
- [ ] テストを実行し、継続して成功することを確認
- [ ] 既存テストが破損していないことを確認

✅ **Phase Complete**

---

## Process 3: 英語トークンに数字を含める

### Red Phase: テスト作成と失敗確認
- [ ] 「vimconf2025」を含むキーワード検索テストを作成
  - テストドキュメントに「vimconf2025」を含める
  - 「vimconf2025」で検索することを期待するテストを記述
  - 「2025」で検索することも期待
- [ ] テストを実行して失敗することを確認（現状、`[A-Za-z]+` で数字が除外されている）

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] /home/takets/repos/digrag/src/tokenizer/japanese.rs の extract_english_tokens 関数を修正
  - 正規表現を `[A-Za-z][A-Za-z0-9]*` に変更（数字を含める）
  - Optional: 先頭が数字の場合の処理を検討
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] 正規表現の妥当性を確認（他のキーワード抽出に影響がないか）
- [ ] テストを実行し、継続して成功することを確認
- [ ] cargo clippy で警告がないことを確認

✅ **Phase Complete**

---

## Process 4: CamelCase分割

### Red Phase: テスト作成と失敗確認
- [ ] 「vim」を含むキーワード検索テストを作成
  - テストドキュメントに「VimConf」を含める
  - 「vim」で検索して「VimConf」が見つかることを期待するテストを記述
  - 「VimConf」そのものでも見つかることを期待
- [ ] テストを実行して失敗することを確認（現状、CamelCase が分割されていない）

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] /home/takets/repos/digrag/src/tokenizer/japanese.rs に split_camel_case 関数を追加
  - 「VimConf2025」を「Vim」「Conf」「2025」に分割
  - 小文字で始まるキャメルケース「myVariable」を「my」「Variable」に分割
- [ ] extract_english_tokens 関数を拡張して split_camel_case を使用
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] split_camel_case のロジックをリファクタリング（複雑なケースへの対応）
- [ ] テストを実行し、継続して成功することを確認
- [ ] cargo clippy で警告がないことを確認
- [ ] 既存テストが全て通過することを確認

✅ **Phase Complete**

---

## Process 5: ハイブリッド検索でembedding client使用

### Red Phase: テスト作成と失敗確認
- [ ] ハイブリッド検索でセマンティック検果が返されることを期待するテストを作成
  - embedding client が設定されていない場合と設定されている場合の違いを検証
  - embedding API key が環境変数から読み込まれることを確認
- [ ] テストを実行して失敗することを確認（現状、embedding client が設定されていない）

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] /home/takets/repos/digrag/src/main.rs の CLI パートを修正
  - 環境変数 `OPENAI_API_KEY` または同等のキーから API キーを読み込む
  - Searcher::with_embedding_client() を使用して embedding client を設定
  - Optional: 環境変数がない場合のエラーハンドリング
- [ ] /home/takets/repos/digrag/src/search/searcher.rs の検索ロジックを確認
  - embedding client がある場合、セマンティック検索を実行
  - embedding client がない場合は、BM25のみの結果を返す
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] エラーハンドリングを改善（API キー取得失敗、embedding サービス不可時）
- [ ] ログ出力を追加（embedding client の初期化状態を可視化）
- [ ] テストを実行し、継続して成功することを確認
- [ ] cargo clippy で警告がないことを確認

✅ **Phase Complete**

---

## Process 10: テスト統合

### Red Phase: テスト作成と失敗確認
- [ ] すべての修正に関連するテストケースを統合
- [ ] 各プロセスのテストが全て通過することを確認

✅ **Phase Complete**

### Green Phase: テストが通過するまで実装を調整
- [ ] cargo test を実行し、全テストが通過することを確認
  - BM25タイトル統合テスト
  - 英数字トークン化テスト
  - CamelCase分割テスト
  - ハイブリッド検索テスト
- [ ] テスト結果に基づいて実装を修正

✅ **Phase Complete**

### Refactor Phase: テスト継続実行確認
- [ ] cargo clippy を実行して警告がないことを確認
- [ ] テスト覆率を確認（新規コードのカバレッジ）
- [ ] テスト継続実行確認

✅ **Phase Complete**

---

## Process 50: インデックス再構築と検証

### Red Phase: 検証計画の策定
- [ ] インデックス再構築手順を確認
  - 既存インデックスをバックアップ
  - digrag build コマンドの実行方法を確認
- [ ] 検証テストケースを準備
  - 「vimconf2025」での検索テスト
  - ハイブリッド検索での セマンティック結果確認

✅ **Phase Complete**

### Green Phase: インデックス再構築と初期検証
- [ ] digrag build コマンドを実行してインデックスを再構築
  - `digrag build --input ~/repos/changelog --output ~/repos/changelog/.rag --with-embeddings`
  - 実行に時間がかかる場合は、進捗を確認
- [ ] 基本的なテストケースで検証
  - BM25検索: 「vimconf2025」で検索
  - ハイブリッド検索: セマンティック結果の取得

✅ **Phase Complete**

### Refactor Phase: 本格的な検証と最適化
- [ ] 複数のキーワードでのテスト
  - 「vim」での検索（CamelCase分割）
  - 「2025」での検索（数字トークン化）
  - 「vimconf」での検索（部分マッチ）
- [ ] ハイブリッド検索の結果品質確認
  - BM25結果との比較
  - セマンティック検索の精度確認
- [ ] パフォーマンス確認（検索時間）
- [ ] 修正があれば実装に反映

✅ **Phase Complete**

---

# Management

## Blockers

| ID | Description | Status | Resolution |
|----|-------------|--------|-----------|
| B1 | embedding API の利用可否 | open | OpenAI API キーの可用性を確認 |
| B2 | インデックス再構築に要する時間 | open | 647ドキュメント規模での所要時間を測定 |

## Lessons

| ID | Insight | Severity | Applied |
|----|---------|----------|---------|
| L1 | チルダ展開はシェルレベルで行われるため、JSON設定に絶対パスを使用すべき | high | ☐ |
| L2 | トークン化プロセスは検索精度に大きく影響する（数字・大文字処理） | high | ☐ |
| L3 | embedding client の初期化失敗はグレースフルに処理すべき（BM25フォールバック） | medium | ☐ |

## Feedback Log

| Date | Type | Content | Status |
|------|------|---------|--------|
| 2025-12-27 | planning | 初期計画作成 | open |

## Completion Checklist
- [ ] Process 1: MCP設定の修正 完了
- [ ] Process 2: タイトルをBM25対象に含める 完了
- [ ] Process 3: 英語トークンに数字を含める 完了
- [ ] Process 4: CamelCase分割 完了
- [ ] Process 5: ハイブリッド検索でembedding client使用 完了
- [ ] Process 10: テスト統合 完了（cargo test 全て通過）
- [ ] Process 50: インデックス再構築と検証 完了
- [ ] cargo clippy で警告なし
- [ ] 既存テスト破損なし
- [ ] 実環境での「vimconf2025」検索成功確認

---

<!--
Process番号規則
- 1-9: 機能実装
- 10-49: テスト拡充
- 50-99: フォローアップ
- 100+: 品質・ドキュメント
-->
