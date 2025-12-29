---
mission_id: digrag-extraction-option
title: "digrag Search コマンドに --extraction オプション追加"
status: planning
progress: 0
phase: planning
tdd_mode: true
blockers: 0
created_at: "2025-12-29"
updated_at: "2025-12-29"
---

# digrag Search コマンドに --extraction オプション追加

## Commander's Intent

### Purpose
Search コマンドで抽出モードを CLI から指定できるようにし、MCP ツール（query_memos）と同等の柔軟性を CLI ユーザーにも提供する。

### End State
- `digrag search --extraction <mode>` で抽出モードを指定可能
- `snippet`, `entry`, `full` の3モードをサポート
- 設定ファイル・環境変数との優先順位が正しく機能
- 既存動作に影響なし

### Key Tasks
1. Search struct に `--extraction` オプションを追加
2. CLI引数と設定のマージロジックを実装
3. ContentExtractor を使用した抽出処理を統合
4. テストを実行して動作確認

### Constraints（禁止事項）
- 既存の CLI オプションの動作を変更しない
- MCP query_memos の動作を変更しない

### Restraints（必須事項）
- 既存の `--mode` オプションのパターンに従う
- 設定優先順位: CLI引数 > 環境変数 > 設定ファイル > デフォルト

---

## Context

### 概要
digrag の MCP ツール（query_memos）では `extraction_mode` パラメータで抽出モードを指定できるが、CLI の Search コマンドにはこのオプションがない。CLI ユーザーも同様の柔軟性を得られるよう、`--extraction` オプションを追加する。

### 抽出モード
| モード | 動作 | 用途 |
|--------|------|------|
| `snippet` | 最初の150文字を抽出 | プレビュー用（デフォルト） |
| `entry` | Changelog エントリ形式で抽出 | 構造化メモ |
| `full` | 全コンテンツ抽出（截断あり） | 詳細確認 |

### 開発のゴール
- CLI から `--extraction` / `-e` オプションで抽出モードを指定
- MCP と同じ `ExtractionStrategy` を使用し、動作を統一

---

## References

| @ref | @target | @test | Description |
|------|---------|-------|-------------|
| src/extract/mod.rs | - | - | ExtractionStrategy, ContentExtractor 定義 |
| src/config/app_config.rs | - | - | extraction_mode() getter |
| - | src/main.rs:425-444 | - | Search struct 定義 |
| - | src/main.rs:717-778 | - | Search コマンド処理 |
| - | - | tests/cli_test.rs | CLI テスト（追加予定） |

---

## Progress Map

| Process | Status | Progress | Phase | Notes |
|---------|--------|----------|-------|-------|
| 1. CLI オプション追加 | pending | ▯▯▯▯▯ | Red | |
| 2. マージロジック実装 | pending | ▯▯▯▯▯ | Red | |
| 3. ContentExtractor 統合 | pending | ▯▯▯▯▯ | Red | |
| 4. 動作確認テスト | pending | ▯▯▯▯▯ | Green | |

---

## Processes

### Process 1: CLI オプション追加

#### Phase: Red
- [ ] Search struct に `extraction` フィールドを追加
- [ ] `#[arg(short = 'e', long)]` アノテーションを付与
- [ ] Commands::Search パターンマッチに変数追加
- [ ] `cargo build` でコンパイル確認

#### 実装詳細

**ファイル**: `src/main.rs` (行425-444付近)

```rust
Search {
    /// Search query
    query: String,

    /// Path to the index directory
    #[arg(short, long)]
    index_dir: Option<String>,

    /// Number of results to return
    #[arg(short, long)]
    top_k: Option<usize>,

    /// Search mode: bm25, semantic, or hybrid
    #[arg(short, long)]
    mode: Option<String>,

    /// Filter by tag
    #[arg(long)]
    tag: Option<String>,

    /// Extraction mode: snippet, entry, or full    // 追加
    #[arg(short = 'e', long)]                       // 追加
    extraction: Option<String>,                      // 追加
}
```

**パターンマッチ**: `src/main.rs` (行717-723付近)

```rust
Commands::Search { query, index_dir, top_k, mode, tag, extraction } => {
```

---

### Process 2: マージロジック実装

#### Phase: Red
- [ ] `effective_extraction` 変数を追加
- [ ] CLI引数と設定ファイルのマージを実装
- [ ] 無効値の場合のフォールバック処理を追加

#### 実装詳細

**ファイル**: `src/main.rs` (行730-731の後)

```rust
let effective_extraction = extraction
    .unwrap_or_else(|| app_config.extraction_mode().to_string());
```

---

### Process 3: ContentExtractor 統合

#### Phase: Red
- [ ] 必要なインポートを追加
- [ ] ExtractionStrategy の決定ロジックを追加
- [ ] TruncationConfig と ContentExtractor を作成
- [ ] 結果表示ロジックを置き換え

#### 実装詳細

**インポート追加**: `src/main.rs` (ファイル冒頭)

```rust
use digrag::extract::{ContentExtractor, ExtractionStrategy, TruncationConfig};
```

**ExtractionStrategy 決定**: `src/main.rs` (行741の後)

```rust
let extraction_strategy = match effective_extraction.as_str() {
    "entry" => ExtractionStrategy::ChangelogEntry,
    "full" => ExtractionStrategy::Full,
    "snippet" => ExtractionStrategy::Head(150),
    _ => {
        eprintln!("Unknown extraction mode '{}', using snippet", effective_extraction);
        ExtractionStrategy::Head(150)
    }
};

let truncation = TruncationConfig {
    max_chars: Some(app_config.extraction_max_chars()),
    max_lines: None,
    max_sections: None,
};
let extractor = ContentExtractor::new(extraction_strategy, truncation);
```

**結果表示**: `src/main.rs` (行766-776付近)

```rust
// 旧: let snippet: String = doc.text.chars().take(100).collect();
// 新:
let extracted = extractor.extract(&doc.text);
println!("   {}", extracted.text);
if extracted.truncated {
    println!("   [truncated: {} of {} chars shown]",
        extracted.stats.extracted_chars,
        extracted.stats.total_chars);
}
```

---

### Process 4: 動作確認テスト

#### Phase: Green
- [ ] `digrag search --help` でオプション表示確認
- [ ] `digrag search "test" -e snippet` でsnippetモード動作確認
- [ ] `digrag search "test" -e entry` でentryモード動作確認
- [ ] `digrag search "test" -e full` でfullモード動作確認
- [ ] `digrag search "test" -e invalid` で警告とフォールバック確認
- [ ] オプション未指定時に設定ファイルの値が使用されることを確認

#### テストコマンド

```bash
# ヘルプ確認
digrag search --help

# 各モード
digrag search "test" -i .rag --extraction snippet
digrag search "test" -i .rag -e entry
digrag search "test" -i .rag -e full

# 無効値（警告 + フォールバック）
digrag search "test" -i .rag -e invalid
```

---

## Management

### Blockers
（なし）

### Lessons
- 既存パターン（`--mode`）に従うことで一貫性のある実装が可能
- MCP と CLI で同じ抽出ロジックを共有することで動作を統一

### Feedback Log
| Timestamp | Source | Feedback | Action |
|-----------|--------|----------|--------|
| | | | |

### Completion Checklist
- [ ] 全 Process が完了
- [ ] `cargo build` 成功
- [ ] `cargo test` 成功
- [ ] 動作確認完了
- [ ] コミットメッセージ作成

---

