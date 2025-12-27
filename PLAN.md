---
mission_id: null
title: "Incremental Build with Content Hashing"
status: planning
progress: 0
phase: planning
tdd_mode: true
blockers: 0
created_at: ""
updated_at: ""
---

# Commander's Intent

## Purpose
- Embedding API コストを削減するため、変更のあるドキュメントのみを処理する
- digrag build コマンドにインクリメンタルビルド機能を追加し、コンテンツハッシュで差分検出を実現

## End State
- `digrag build --incremental` でインクリメンタルビルド実行可能
- 変更のないドキュメントの Embedding 再生成をスキップ
- 削除されたドキュメントはインデックスから自動削除
- 既存インデックスとの後方互換性を保証

## Key Tasks
- ドキュメント ID をコンテンツハッシュベースに変更（再現性確保）
- IncrementalDiff の実装（追加・変更・削除の判定）
- IndexMetadata にスキーマバージョンとハッシュ情報を追加
- VectorIndex/Docstore に削除メソッドを実装
- IndexBuilder でインクリメンタルビルド機能を実装
- CLI に --incremental フラグを追加
- 統合テストの作成

## Constraints
- 既存のドキュメント ID（UUID v4）の生成ロジックは変更して良い（下位互換性不要）
- スキーマバージョン "1.0" 以前の場合は必ずフルリビルドにフォールバック
- コンテンツハッシュは title + "\0" + text のみを対象（メタデータは影響させない）

## Restraints
- すべての変更は TDD（Red → Green → Refactor）で実装
- テスト完了なしに実装コードをマージしない
- Embedding API 削減効果を測定可能にする（出力メッセージに件数を含める）

---

# Context

## 概要
- インクリメンタルビルド機能により、再度ビルドする際に変更のあるドキュメントのみ新規 Embedding を生成
- ユーザーは従来通り `digrag build` を実行するだけで、自動的に差分検出が行われ、効率的にインデックスが更新される
- Embedding API の呼び出し回数を大幅に削減し、コスト削減と速度向上を実現

## 必須のルール
- 必ず `CLAUDE.md` を参照し、ルールを守ること
- **TDD（テスト駆動開発）を厳守すること**
  - 各プロセスは必ずテストファーストで開始する（Red → Green → Refactor）
  - 実装コードを書く前に、失敗するテストを先に作成する
  - テストが通過するまで修正とテスト実行を繰り返す
  - プロセス完了の条件：該当するすべてのテスト、フォーマッタ、Linter が通過していること

## 開発のゴール
- Embedding API コスト削減（変更がないドキュメントの再生成をスキップ）
- 大規模なドキュメントセット（640+）のビルド時間を短縮
- ドキュメント追加・更新・削除に対応した柔軟なインデックス管理

---

# References

| @ref | @target | @test |
|------|---------|-------|
| src/loader/document.rs | src/loader/document.rs | tests/test_document_hashing.rs |
| src/index/builder.rs | src/index/diff.rs | tests/test_incremental_diff.rs |
| src/index/builder.rs | src/index/builder.rs | tests/test_incremental_build.rs |
| src/index/vector.rs | src/index/vector.rs | tests/test_vector_index_remove.rs |
| src/index/docstore.rs | src/index/docstore.rs | tests/test_docstore_remove.rs |
| src/main.rs | src/main.rs | tests/test_cli_incremental.rs |

---

# Progress Map

| Process | Status | Progress | Phase | Notes |
|---------|--------|----------|-------|-------|
| Process 1 | planning | ▯▯▯▯▯ 0% | Red | コンテンツハッシュ機能の実装 |
| Process 2 | planning | ▯▯▯▯▯ 0% | Red | IncrementalDiff の実装 |
| Process 3 | planning | ▯▯▯▯▯ 0% | Red | IndexMetadata の拡張 |
| Process 4 | planning | ▯▯▯▯▯ 0% | Red | Docstore/VectorIndex の削除機能 |
| Process 5 | planning | ▯▯▯▯▯ 0% | Red | IndexBuilder のインクリメンタル対応 |
| Process 6 | planning | ▯▯▯▯▯ 0% | Red | CLI オプション追加 |
| Process 10 | planning | ▯▯▯▯▯ 0% | Red | 統合テスト作成 |
| Process 200 | planning | ▯▯▯▯▯ 0% | Red | ドキュメンテーション |
| Process 300 | planning | ▯▯▯▯▯ 0% | Red | OODAフィードバックループ |
| | | | | |
| **Overall** | **planning** | **▯▯▯▯▯ 0%** | **planning** | **Blockers: 0** |

---

# Processes

## Process 1: コンテンツハッシュ機能の実装

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/test_document_hashing.rs` を作成
  - `compute_content_hash("title", "text")` が SHA256[:16] を返すこと
  - 同じコンテンツは同じハッシュを返すこと（再現性）
  - 異なるコンテンツは異なるハッシュを返すこと
  - メタデータ（date, tags）の変更はハッシュに影響しないこと
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] `Cargo.toml` に依存を追加
  - `sha2 = "0.10"`
  - `hex = "0.4"`
- [ ] `src/loader/document.rs` に `compute_content_hash()` メソッドを実装
  - 入力: title, text
  - 処理: SHA256(title + "\0" + text)
  - 出力: 16文字の hex 文字列
- [ ] `with_content_id()` コンストラクタを追加（ID生成をコンテンツハッシュ化）
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] ハッシュ生成ロジックの最適化
- [ ] ドキュメント構造体のコメントを追加
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

## Process 2: IncrementalDiff の実装

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/test_incremental_diff.rs` を作成
  - 新しいドキュメント → added に分類されること
  - 既存ドキュメント（コンテンツ変更なし） → unchanged に分類されること
  - 既存ドキュメント（コンテンツ変更あり） → modified に分類されること
  - インデックスに存在しない ID → removed に分類されること
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] `src/index/diff.rs` を新規作成
  ```rust
  pub struct IncrementalDiff {
      pub added: Vec<Document>,
      pub modified: Vec<Document>,
      pub removed: Vec<String>,
      pub unchanged: Vec<String>,
  }

  impl IncrementalDiff {
      pub fn compute(
          new_docs: Vec<Document>,
          existing_index: &IndexMetadata,
      ) -> Self { ... }
  }
  ```
- [ ] `src/index/mod.rs` に `mod diff;` を追加
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] エラーハンドリングの強化
- [ ] Edge Case（空リスト、全削除など）への対応確認
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

## Process 3: IndexMetadata の拡張

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/test_index_metadata.rs` を作成
  - `schema_version` フィールドが存在すること
  - `doc_hashes: HashMap<String, String>` が存在すること
  - メタデータの読み書きが正しく動作すること
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] `src/index/builder.rs` の `IndexMetadata` を拡張
  ```rust
  pub struct IndexMetadata {
      // 既存フィールド...
      pub schema_version: String,  // "2.0"
      pub doc_hashes: HashMap<String, String>,  // doc_id -> content_hash
  }
  ```
- [ ] 既存の metadata.json との互換性チェックロジックを実装
  - `schema_version` が "1.0" 以下 → フルリビルド
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] デフォルト値の確認
- [ ] シリアライゼーション/デシリアライゼーションの検証
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

## Process 4: Docstore/VectorIndex の削除機能実装

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/test_docstore_remove.rs` を作成
  - `remove(doc_id)` で指定 ID のドキュメントが削除されること
  - `remove_batch(doc_ids)` で複数削除が正しく動作すること
  - 存在しない ID の削除でも問題が生じないこと
- [ ] テストファイル `tests/test_vector_index_remove.rs` を作成
  - `remove(doc_id)` でベクトルが削除されること
  - `remove_batch(doc_ids)` で複数削除が正しく動作すること
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] `src/index/docstore.rs` に以下を追加
  ```rust
  pub fn remove(&mut self, doc_id: &str) {
      self.documents.remove(doc_id);
  }

  pub fn remove_batch(&mut self, doc_ids: Vec<String>) {
      for doc_id in doc_ids {
          self.documents.remove(&doc_id);
      }
  }
  ```
- [ ] `src/index/vector.rs` に以下を追加
  ```rust
  pub fn remove(&mut self, doc_id: &str) { ... }
  pub fn remove_batch(&mut self, doc_ids: Vec<String>) { ... }
  ```
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] エラーハンドリング強化
- [ ] Batch削除の効率化
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

## Process 5: IndexBuilder のインクリメンタルビルド実装

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/test_incremental_build.rs` を作成
  - フルビルド → インクリメンタルビルド の流れが正しく動作すること
  - added/modified ドキュメントのみ embedding が生成されること
  - removed ドキュメントがインデックスから削除されること
  - unchanged ドキュメントは変更されないこと
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] `src/index/builder.rs` に以下を追加
  ```rust
  pub async fn build_incremental(
      &mut self,
      documents: Vec<Document>,
      existing_metadata: &IndexMetadata,
      embedding_model: &str,
  ) -> Result<IndexMetadata, BuildError> {
      // IncrementalDiff を計算
      // added/modified のみ embedding 生成
      // removed を docstore/vector から削除
      // metadata を更新して返す
  }

  pub fn load_existing_index(output_dir: &Path) -> Result<IndexMetadata> {
      // metadata.json を読み込み
      // スキーマバージョン確認
  }
  ```
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] エラーハンドリング強化
- [ ] 既存インデックス読み込みロジックの安定性向上
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

## Process 6: CLI オプション追加

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/test_cli_incremental.rs` を作成
  - `--incremental` フラグが認識されること
  - `--force` フラグでフルリビルドが強制されること
  - フラグなしの場合は既存動作を保持すること
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

### Green Phase: 最小実装と成功確認
- [ ] `src/main.rs` の build コマンド定義に以下を追加
  ```rust
  #[arg(long)]
  incremental: bool,

  #[arg(long)]
  force: bool,  // --incremental でもフルリビルド
  ```
- [ ] build コマンドハンドラーに条件分岐を追加
  - `incremental && !force` → インクリメンタルビルド
  - その他 → フルビルド
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

### Refactor Phase: 品質改善と継続成功確認
- [ ] ヘルプテキストの追加
- [ ] エラーハンドリング強化
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

## Process 10: 統合テスト作成

### Red Phase: テスト作成と失敗確認
- [ ] テストファイル `tests/integration_test_incremental.rs` を作成
  - E2E テスト: ドキュメント追加 → ビルド → 変更 → インクリメンタルビルド → 確認
  - 出力メッセージに "Embeddings generated: X" が含まれることを確認

✅ **Phase Complete**

### Green Phase: テストが通過するまで実装を調整
- [ ] E2E テストを実行して成功することを確認
- [ ] 出力メッセージのフォーマットを確認
  ```
  Incremental build complete:
    Added: X documents
    Modified: Y documents
    Removed: Z documents
    Unchanged: W documents
    Embeddings generated: A  # ← APIコスト発生は A 件のみ
  ```

✅ **Phase Complete**

### Refactor Phase: テスト継続実行確認
- [ ] すべてのテストが通過することを確認
- [ ] 統合テストと単体テストの連携を確認

✅ **Phase Complete**

---

## Process 200: ドキュメンテーション

### Red Phase: ドキュメント設計
- [ ] 文書化対象を特定
  - インクリメンタルビルド機能の説明
  - コンテンツハッシュの仕組み
  - CLI 使用例
  - 後方互換性について
- [ ] ドキュメント構成を作成
- [ ] **成功条件**: アウトラインが存在

✅ **Phase Complete**

### Green Phase: ドキュメント記述
- [ ] README.md に "インクリメンタルビルド" セクションを追加
- [ ] src/index/diff.rs に API ドキュメントコメントを追加
- [ ] CLI 使用例を README に追加
  ```bash
  # 初回: フルビルド
  digrag build --input changelog.md --output ~/.rag --with-embeddings

  # 2回目以降: インクリメンタル
  digrag build --input changelog.md --output ~/.rag --with-embeddings --incremental

  # 強制フルリビルド
  digrag build --input changelog.md --output ~/.rag --with-embeddings --incremental --force
  ```
- [ ] **成功条件**: 全doc_targetsがカバー済み、Markdown構文正常

✅ **Phase Complete**

### Refactor Phase: 品質確認
- [ ] 一貫性チェック（用語・フォーマット統一）
- [ ] リンク検証（リンク切れなし）
- [ ] **成功条件**: 全レポートOK

✅ **Phase Complete**

---

## Process 300: OODAフィードバックループ（教訓・知見の保存）

### Red Phase: フィードバック収集設計

**Observe（観察）**
- [ ] 実装過程で発生した問題・課題を収集
- [ ] テスト結果から得られた知見を記録
- [ ] コードレビューのフィードバックを整理

**Orient（方向付け）**
- [ ] 収集した情報をカテゴリ別に分類
  - Technical: Rust での ハッシュ計算、インデックス管理
  - Process: TDD プロセスの有効性、テスト設計
  - Antipattern: 避けるべきパターン
  - Best Practice: 推奨パターン
- [ ] **成功条件**: 収集対象が特定され、分類基準が明確

✅ **Phase Complete**

### Green Phase: 教訓・知見の永続化

**Decide（決心）**
- [ ] 保存すべき教訓・知見を選定
- [ ] 各項目の保存先を決定

**Act（行動）**
- [ ] コードに関する知見をMarkdownで記録
- [ ] 関連するコード箇所にコメントを追加（必要に応じて）
- [ ] **成功条件**: 全教訓がstigmergyに保存済み

✅ **Phase Complete**

### Refactor Phase: フィードバック品質改善

**Feedback Loop**
- [ ] 保存した教訓の品質を検証
- [ ] メタ学習: OODAプロセス自体の改善点を記録

**Cross-Feedback**
- [ ] 他のProcess（100, 200）との連携を確認
- [ ] **成功条件**: 教訓がstigmergyで検索可能

✅ **Phase Complete**

---

# Management

## Blockers

| ID | Description | Status | Resolution |
|----|-------------|--------|-----------|
| | No blockers yet | - | - |

## Lessons

| ID | Insight | Severity | Applied |
|----|---------|----------|---------|
| | To be filled during implementation | - | ☐ |

## Feedback Log

| Date | Type | Content | Status |
|------|------|---------|--------|
| | | | |

## Completion Checklist
- [ ] すべてのProcess完了
- [ ] すべてのテスト合格
- [ ] コードレビュー完了
- [ ] ドキュメント更新完了
- [ ] マージ可能な状態

---

<!--
Process番号規則
- 1-9: 機能実装
- 10-49: テスト拡充
- 50-99: フォローアップ
- 100-199: 品質向上（リファクタリング）
- 200-299: ドキュメンテーション
- 300+: OODAフィードバックループ（教訓・知見保存）
-->

