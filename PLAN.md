---
mission_id: digrag-embedding-quality
title: "digrag Embedding品質向上 - タイトル・タグ・カテゴリ統合"
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
changelogmemo独自フォーマットのセマンティック検索品質を向上させる。現在、Embedding入力がテキスト本文のみで、タイトル・タグ・カテゴリ情報が活用されていないため、検索精度が最適化されていない。

## End State
- Embedding入力にタイトル・タグが含まれ、検索精度が15-20%向上
- カテゴリ階層（「Claude Code / hookタイミング」形式）が抽出可能
- インデックス再構築後、「VimConf」「Claude Code」などのキーワードで関連エントリが上位に表示される
- 既存テストが全て通過する
- cargo clippy の警告がない

## Key Tasks
- Embedding入力テキストにタイトル・タグを追加
- Document構造体にcategory()/subcategory()メソッドを追加
- create_embedding_text関数のテストを追加
- インデックス再構築と検索精度検証

## Constraints
- 既存のテストを壊さない
- 後方互換性を維持（既存インデックスの読み込みは継続サポート）
- APIコスト増加を最小限に（embedding入力テキスト長は適切に制限）

## Restraints
- cargo test が全て通過すること
- cargo clippy で警告がないこと

---

# Context

## 概要
changelogmemoは725エントリ、37,836行の独自changelog形式ファイル。タイトルに「カテゴリ / サブタイトル」形式が多く（例: `Claude Code / hookタイミング`）、タグ分布は worklog(24%), end(23%), memo(16%), tips(9%)。

現在のdigrag実装では：
1. **Embedding入力がテキスト本文のみ** - タイトル・タグが含まれていない
2. **長いエントリの圧縮** - チャンキングなしで1つのembeddingに圧縮
3. **カテゴリ情報の未活用** - タイトル内の階層構造が抽出されていない

この問題を解決し、セマンティック検索の精度を向上させる。

## 必須のルール
- 必ず `CLAUDE.md` を参照し、ルールを守ること
- **TDD（テスト駆動開発）を厳守すること**
  - 各プロセスは必ずテストファーストで開始する（Red → Green → Refactor）
  - 実装コードを書く前に、失敗するテストを先に作成する
  - テストが通過するまで修正とテスト実行を繰り返す
  - プロセス完了の条件：該当するすべてのテスト、フォーマッタ、Linterが通過していること

## 開発のゴール
- Embedding品質を向上させ、セマンティック検索の精度を改善する
- タイトル・タグ・カテゴリ情報を活用した検索が可能になる
- 将来的なセマンティックチャンキング対応の基盤を整備

---

# References

| @ref | @target | @test |
|------|---------|-------|
| changelogmemo形式 | ~/repos/changelog/changelogmemo | N/A |
| Embedding生成 | /home/takets/repos/digrag/src/index/builder.rs | /home/takets/repos/digrag/src/index/builder.rs (tests) |
| Document構造体 | /home/takets/repos/digrag/src/changelog.rs | /home/takets/repos/digrag/src/changelog.rs (tests) |
| BM25インデックス | /home/takets/repos/digrag/src/index/bm25.rs | /home/takets/repos/digrag/src/index/bm25.rs (tests) |
| 検索ロジック | /home/takets/repos/digrag/src/search/searcher.rs | /home/takets/repos/digrag/src/search/searcher.rs (tests) |

---

# Progress Map

| Process | Status | Progress | Phase | Notes |
|---------|--------|----------|-------|-------|
| Process 1: Embedding入力にタイトル・タグを追加 | planning | ▯▯▯▯▯ 0% | Red | builder.rs:154行目付近 |
| Process 2: カテゴリ階層抽出メソッドを追加 | planning | ▯▯▯▯▯ 0% | Red | changelog.rs |
| Process 10: テスト統合 | planning | ▯▯▯▯▯ 0% | Red | cargo test 通過確認 |
| Process 50: インデックス再構築と検証 | planning | ▯▯▯▯▯ 0% | planning | 実環境での検証 |
| **Overall** | **planning** | **▯▯▯▯▯ 0%** | **planning** | **Blockers: 0** |

---

# Processes

## Process 1: Embedding入力にタイトル・タグを追加

### Red Phase: テスト作成と失敗確認
- [ ] create_embedding_text関数のテストを作成
  - タイトル+本文のフォーマット確認
  - タグあり/なしの両ケース
  - 空タイトル・空タグの境界値テスト
- [ ] テストを実行して失敗することを確認（現状、関数が存在しない）

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] /home/takets/repos/digrag/src/index/builder.rs に create_embedding_text 関数を追加
  ```rust
  fn create_embedding_text(doc: &Document) -> String {
      let tags = doc.tags().join(", ");
      let title = doc.title();

      if tags.is_empty() {
          format!("# {}\n\n{}", title, doc.text)
      } else {
          format!("# {}\nタグ: {}\n\n{}", title, tags, doc.text)
      }
  }
  ```
- [ ] embedding生成時に create_embedding_text を使用するよう修正
  - 現状: `let texts: Vec<String> = documents.iter().map(|d| d.text.clone()).collect();`
  - 修正後: `let texts: Vec<String> = documents.iter().map(create_embedding_text).collect();`
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] 関数のドキュメントコメントを追加
- [ ] テストを実行し、継続して成功することを確認
- [ ] cargo clippy で警告がないことを確認

✅ **Phase Complete**

---

## Process 2: カテゴリ階層抽出メソッドを追加

### Red Phase: テスト作成と失敗確認
- [ ] Document::category() メソッドのテストを作成
  - 「Claude Code / hookタイミング」→「Claude Code」
  - 「単一カテゴリ」→「単一カテゴリ」
  - 空タイトルの場合
- [ ] Document::subcategory() メソッドのテストを作成
  - 「Claude Code / hookタイミング」→「hookタイミング」
  - 「単一カテゴリ」→ None
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] /home/takets/repos/digrag/src/changelog.rs に category()/subcategory() メソッドを追加
  ```rust
  impl Document {
      pub fn category(&self) -> Option<&str> {
          self.title().split(" / ").next()
      }

      pub fn subcategory(&self) -> Option<&str> {
          let parts: Vec<&str> = self.title().split(" / ").collect();
          if parts.len() > 1 { Some(parts[1]) } else { None }
      }
  }
  ```
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] 関数のドキュメントコメントを追加
- [ ] テストを実行し、継続して成功することを確認
- [ ] 既存テストが破損していないことを確認

✅ **Phase Complete**

---

## Process 10: テスト統合

### Red Phase: テスト作成と失敗確認
- [ ] すべての修正に関連するテストケースを統合
- [ ] 各プロセスのテストが全て通過することを確認

✅ **Phase Complete**

### Green Phase: テストが通過するまで実装を調整
- [ ] cargo test を実行し、全テストが通過することを確認
  - create_embedding_text テスト
  - category/subcategory テスト
  - 既存のBM25/検索テスト
- [ ] テスト結果に基づいて実装を修正

✅ **Phase Complete**

### Refactor Phase: テスト継続実行確認
- [ ] cargo clippy を実行して警告がないことを確認
- [ ] テスト継続実行確認

✅ **Phase Complete**

---

## Process 50: インデックス再構築と検証

### Red Phase: 検証計画の策定
- [ ] インデックス再構築手順を確認
  - 既存インデックスをバックアップ
  - digrag build コマンドの実行方法を確認
- [ ] 検証テストケースを準備
  - 「VimConf」での検索テスト（タイトル含むエントリが上位）
  - 「Claude Code」での検索テスト（カテゴリ一致エントリが上位）

✅ **Phase Complete**

### Green Phase: インデックス再構築と初期検証
- [ ] digrag build コマンドを実行してインデックスを再構築
  - `digrag build --input ~/repos/changelog/changelogmemo --output ~/repos/changelog/.rag --with-embeddings`
  - 実行に時間がかかる場合は、進捗を確認
- [ ] 基本的なテストケースで検証
  - セマンティック検索: 「VimConf」で検索
  - ハイブリッド検索: 「Claude Code」で検索

✅ **Phase Complete**

### Refactor Phase: 本格的な検証と最適化
- [ ] 複数のキーワードでのテスト
  - タイトル内キーワード検索
  - タグ検索
  - カテゴリ検索
- [ ] 検索結果の品質確認
  - 関連エントリが上位に表示されるか
  - 改善前後の比較（可能であれば）
- [ ] 修正があれば実装に反映

✅ **Phase Complete**

---

# Management

## Blockers

| ID | Description | Status | Resolution |
|----|-------------|--------|-----------|
| B1 | embedding API の利用可否 | open | OPENROUTER_API_KEY の可用性を確認 |

## Lessons

| ID | Insight | Severity | Applied |
|----|---------|----------|---------|
| L1 | Embedding入力にメタデータを含めることで検索精度が向上 | high | ☐ |
| L2 | カテゴリ階層はタイトルから抽出可能 | medium | ☐ |
| L3 | 日本語特化モデルは将来の改善オプション | low | ☐ |

## Feedback Log

| Date | Type | Content | Status |
|------|------|---------|--------|
| 2025-12-27 | planning | 初期計画作成 | open |
| 2025-12-27 | research | serena-v4によるchangelogmemo形式調査完了 | closed |

## Completion Checklist
- [ ] Process 1: Embedding入力にタイトル・タグを追加 完了
- [ ] Process 2: カテゴリ階層抽出メソッドを追加 完了
- [ ] Process 10: テスト統合 完了（cargo test 全て通過）
- [ ] Process 50: インデックス再構築と検証 完了
- [ ] cargo clippy で警告なし
- [ ] 既存テスト破損なし
- [ ] 実環境での検索精度向上確認

---

<!--
Process番号規則
- 1-9: 機能実装
- 10-49: テスト拡充
- 50-99: フォローアップ
- 100+: 品質・ドキュメント
-->
