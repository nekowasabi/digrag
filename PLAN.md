---
mission_id: content-extraction-feature
title: ベクター検索後のコンテンツ取得範囲指定機能
status: planning
phase: design
created: 2025-12-28
---

# ベクター検索後のコンテンツ取得範囲指定機能

## Commander's Intent

### 目的
changelogmemoでベクター検索後、`*`ヘッダから始まる記事範囲を抽出し、要約＋生データを返す機能を実装する。

### 完了状態
- [ ] プロンプトで取得範囲を指定できる（MCPパラメータ + LLM動的判断）
- [ ] `*`から始まるchangelogエントリ単位で抽出可能
- [ ] テキスト量が多すぎる場合は自動トランケーション
- [ ] 要約（ルールベース/LLM選択可能）と生データを両方返却
- [ ] 設定ファイル（config.toml）で各種パラメータを設定可能
- [ ] OpenRouterプロバイダーのルーティング設定が可能

### 主要タスク
1. コンテンツ抽出エンジン（`src/extract/`）の新規作成
2. 設定ファイル拡張（抽出設定 + LLM要約設定 + プロバイダー設定）
3. MCPツール拡張（パラメータ追加、出力形式変更）
4. Searcherへの統合

### 制約
- 既存のスニペット機能（先頭150文字）との後方互換性を維持
- デフォルトはルールベース要約（APIコスト節約）
- LLMモデル・プロバイダーはconfig.tomlでのみ設定（MCP経由での変更不可）

### 必須事項
- 設定優先順位: MCPパラメータ > 環境変数 > config.toml > デフォルト値
- OpenRouter形式のモデル指定: `provider/model`（例: `cerebras/llama-3.3-70b`）
- プロバイダールーティング: `order`, `allow_fallbacks`, `only`等をサポート

---

## Context

### 概要
digragはchangelogmemo形式のテキストをRAG検索するCLI/MCPサーバー。現在は検索結果の先頭150文字のみをスニペットとして返却している。本機能では、`*`ヘッダで区切られたエントリ全体を抽出し、要約と生データを組み合わせて返却する。

### 必須ルール
- 後方互換性: `extraction_mode = "snippet"` がデフォルト
- コスト管理: LLM要約はオプション、デフォルト無効
- トランケーション: デフォルト5000文字

### 開発ゴール
MCPクライアント（Claude等）が、検索結果のコンテンツを適切な粒度で取得できるようにする。

---

## References

| @ref | @target | @test |
|------|---------|-------|
| `src/config/app_config.rs` | 抽出設定・LLM設定・プロバイダー設定追加 | - |
| `src/extract/mod.rs` | **新規作成** - 抽出エンジン基本構造 | `tests/extract_mod.rs` |
| `src/extract/changelog.rs` | **新規作成** - changelogエントリ抽出器 | `tests/extract_changelog.rs` |
| `src/extract/summarizer.rs` | **新規作成** - 要約生成器（ルール/LLM） | `tests/extract_summarizer.rs` |
| `src/search/searcher.rs` | 抽出器呼び出し統合 | - |
| `src/main.rs` | MCPパラメータ・出力形式拡張 | - |
| `src/lib.rs` | extractモジュール追加 | - |

---

## Progress Map

| Process | Status | Description |
|---------|--------|-------------|
| 1 | ✅ | 設定ファイル拡張（app_config.rs） |
| 2 | ✅ | 抽出エンジン基本構造（extract/mod.rs） |
| 3 | ✅ | changelogエントリ抽出器（extract/changelog.rs） |
| 4 | ✅ | 要約生成器（extract/summarizer.rs） |
| 5 | ✅ | lib.rsへのモジュール追加 |
| 6 | ✅ | Searcher統合（searcher.rs） |
| 7 | ✅ | MCPツール拡張（main.rs） |
| 8 | ✅ | OpenRouter HTTPクライアント実装 |
| 9 | ✅ | LLM要約API呼び出し実装 |
| 10 | ✅ | 単体テスト作成 |
| 11 | ✅ | 統合テスト作成 |
| 200 | ✅ | 長期改善: キャッシュ・レート制限・フォールバック |
| 300 | ✅ | フィードバック収集: 使用統計・エラー分析 |

---

## Processes

### Process 1: 設定ファイル拡張

#### 設計

**`src/config/app_config.rs` への追加フィールド:**

```rust
pub struct AppConfig {
    // 既存フィールド
    pub index_dir: String,
    pub openrouter_api_key: Option<String>,
    pub default_top_k: usize,
    pub default_search_mode: String,

    // 新規: コンテンツ抽出設定
    pub extraction_mode: String,           // "snippet" | "entry" | "full"
    pub extraction_max_chars: usize,       // デフォルト: 5000
    pub extraction_include_summary: bool,  // デフォルト: true
    pub extraction_include_raw: bool,      // デフォルト: true

    // 新規: LLM要約設定
    pub summarization_enabled: bool,       // デフォルト: false
    pub summarization_model: String,       // デフォルト: "cerebras/llama-3.3-70b"
    pub summarization_max_tokens: usize,   // デフォルト: 500
    pub summarization_temperature: f32,    // デフォルト: 0.3

    // 新規: OpenRouterプロバイダー設定
    pub provider_order: Option<Vec<String>>,      // 優先プロバイダー順序
    pub provider_allow_fallbacks: bool,           // フォールバック許可（デフォルト: true）
    pub provider_only: Option<Vec<String>>,       // 許可するプロバイダーのみ
    pub provider_ignore: Option<Vec<String>>,     // 無視するプロバイダー
    pub provider_sort: Option<String>,            // "price" | "throughput"
    pub provider_require_parameters: bool,        // パラメータ完全サポート必須
}
```

**config.toml 設定例:**

```toml
# ~/.config/digrag/config.toml

# 既存設定
index_dir = ".rag"
default_top_k = 10
default_search_mode = "hybrid"

# コンテンツ抽出設定
extraction_mode = "entry"
extraction_max_chars = 5000
extraction_include_summary = true
extraction_include_raw = true

# LLM要約設定
summarization_enabled = true
summarization_model = "cerebras/llama-3.3-70b"
summarization_max_tokens = 500
summarization_temperature = 0.3

# OpenRouterプロバイダー設定
provider_order = ["Cerebras", "Together"]
provider_allow_fallbacks = true
# provider_only = ["Cerebras"]  # 特定プロバイダーのみ許可
# provider_ignore = ["OpenAI"]  # 特定プロバイダーを除外
# provider_sort = "price"       # 価格順でソート
```

**環境変数対応:**

| 設定項目 | 環境変数 |
|---------|---------|
| extraction_mode | DIGRAG_EXTRACTION_MODE |
| extraction_max_chars | DIGRAG_EXTRACTION_MAX_CHARS |
| summarization_enabled | DIGRAG_SUMMARIZATION_ENABLED |
| summarization_model | DIGRAG_SUMMARIZATION_MODEL |
| provider_order | DIGRAG_PROVIDER_ORDER (カンマ区切り) |
| provider_allow_fallbacks | DIGRAG_PROVIDER_ALLOW_FALLBACKS |

---

### Process 2: 抽出エンジン基本構造

**`src/extract/mod.rs`:**

```rust
pub mod changelog;
pub mod summarizer;

/// 抽出戦略
pub enum ExtractionStrategy {
    /// 先頭N文字（従来互換）
    Head(usize),
    /// changelogエントリ単位
    ChangelogEntry,
    /// 全文
    Full,
    /// 正規表現パターン（将来拡張）
    Pattern { start: Regex, end: Option<Regex> },
}

/// トランケーション設定
pub struct TruncationConfig {
    pub max_chars: Option<usize>,
    pub max_lines: Option<usize>,
    pub max_sections: Option<usize>,
}

/// 抽出結果
pub struct ExtractedContent {
    pub text: String,
    pub truncated: bool,
    pub stats: ContentStats,
}

pub struct ContentStats {
    pub total_chars: usize,
    pub total_lines: usize,
    pub extracted_chars: usize,
}

/// コンテンツ抽出器
pub struct ContentExtractor {
    strategy: ExtractionStrategy,
    truncation: TruncationConfig,
}

impl ContentExtractor {
    pub fn new(strategy: ExtractionStrategy, truncation: TruncationConfig) -> Self;
    pub fn extract(&self, full_text: &str) -> ExtractedContent;
}
```

---

### Process 3: changelogエントリ抽出器

**`src/extract/changelog.rs`:**

```rust
/// changelogエントリ抽出器
pub struct ChangelogEntryExtractor {
    /// エントリヘッダーパターン: ^\\* .+ \\d{4}-\\d{2}-\\d{2}
    entry_pattern: Regex,
    truncation: TruncationConfig,
}

impl ChangelogEntryExtractor {
    /// 指定タイトルを含むエントリを抽出
    pub fn extract_by_title(&self, text: &str, title: &str) -> Option<ExtractedContent>;

    /// 全エントリをパースしてリスト化
    pub fn parse_entries(&self, text: &str) -> Vec<ChangelogEntry>;
}

pub struct ChangelogEntry {
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
    pub content: String,
    pub start_offset: usize,
    pub end_offset: usize,
}
```

**アルゴリズム:**
1. `^\\* `で始まる行を全てインデックス化
2. 各エントリの範囲を特定（現在の`*`から次の`*`の前まで）
3. タイトルマッチングでエントリを特定
4. トランケーション適用

---

### Process 4: 要約生成器

**`src/extract/summarizer.rs`:**

```rust
/// 要約戦略
pub enum SummarizationStrategy {
    /// ルールベース（先頭N文字 + 統計）
    RuleBased { preview_chars: usize },
    /// LLMベース
    LlmBased {
        model: String,
        max_tokens: usize,
        temperature: f32,
        provider_config: ProviderConfig,
    },
}

/// OpenRouterプロバイダー設定
pub struct ProviderConfig {
    pub order: Option<Vec<String>>,
    pub allow_fallbacks: bool,
    pub only: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
    pub sort: Option<String>,
    pub require_parameters: bool,
}

impl ProviderConfig {
    /// APIリクエスト用のJSONオブジェクトを生成
    pub fn to_json(&self) -> serde_json::Value;
}

/// 要約生成器
pub struct ContentSummarizer {
    strategy: SummarizationStrategy,
    api_key: Option<String>,
}

impl ContentSummarizer {
    pub async fn summarize(&self, content: &ExtractedContent) -> Summary;
}

pub struct Summary {
    pub text: String,
    pub method: String,  // "rule-based" | "llm"
    pub stats: ContentStats,
}
```

**LLM要約APIリクエスト例:**

```rust
let request_body = json!({
    "model": "cerebras/llama-3.3-70b",
    "messages": [
        {"role": "system", "content": "以下のテキストを簡潔に要約してください。"},
        {"role": "user", "content": content}
    ],
    "max_tokens": 500,
    "temperature": 0.3,
    "provider": {
        "order": ["Cerebras", "Together"],
        "allow_fallbacks": true
    }
});
```

---

### Process 7: MCPツール拡張

**`src/main.rs` - QueryMemosParams拡張:**

```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct QueryMemosParams {
    query: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
    tag_filter: Option<String>,
    #[serde(default = "default_mode")]
    mode: String,

    // 新規パラメータ
    /// コンテンツ抽出モード: "snippet", "entry", "full"
    extraction_mode: Option<String>,
    /// 最大文字数制限
    max_chars: Option<usize>,
    /// 要約を含めるか
    include_summary: Option<bool>,
    /// 生データを含めるか
    include_raw: Option<bool>,
    /// LLM要約を使用するか
    use_llm_summary: Option<bool>,
}
```

**出力形式:**

```
Found 2 results for 'Claude Code':

1. [score: 0.9234] Claude Code / hookタイミング
   Date: 2025-12-27
   Tags: [memo, dev]

   ## Summary (rule-based)
   hookタイミングに関する調査結果。pre-commit時の挙動について...
   [Stats: 1234文字, 45行, 抽出: 1000文字]

   ## Content
   * Claude Code / hookタイミング 2025-12-27 15:30:00 [memo]:[dev]:
   ・pre-commit hookのタイミング調査
   ・git add実行後、commit前に発火
   ...
   [truncated: 5000/12345 chars]

2. [score: 0.8765] ...
```

---

### Process 8: OpenRouter HTTPクライアント実装

**目的:** reqwest と serde_json を使用したOpenRouter API HTTPクライアント基盤の構築
実装対象: `src/extract/summarizer.rs` 内の `llm_summary()` メソッドの完全実装

#### Red Phase: テスト作成と失敗確認
- [ ] HTTPクライアント初期化テスト作成（auth header, base URL確認）
- [ ] OpenRouter APIレスポンス解析テスト作成
- [ ] エラーハンドリングテスト作成（API failure, timeout, invalid response）
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

#### Green Phase: 最小実装と成功確認
- [ ] `reqwest::Client` でhttps://openrouter.ai/api/v1/chat/completions へPOSTリクエスト実装
- [ ] Authorization header に Bearer token 設定
- [ ] request body のJSON形式化（model, messages, max_tokens, temperature, provider）
- [ ] レスポンスパース実装（`choices[0].message.content` 抽出）
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

#### Refactor Phase: 品質改善と継続成功確認
- [ ] エラーメッセージの改善（API error vs network error の区別）
- [ ] retry logic 検討（optional で実装）
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

### Process 9: LLM要約API呼び出し実装

**目的:** `src/extract/summarizer.rs` の `llm_based_summary()` 実装完了とE2Eテスト

#### Red Phase: テスト作成と失敗確認
- [ ] LLM要約呼び出しE2Eテスト作成（mock API使用）
- [ ] config.toml 反映テスト作成
- [ ] テストを実行して失敗することを確認

✅ **Phase Complete**

#### Green Phase: 最小実装と成功確認
- [ ] `llm_summary()` メソッド完全実装
- [ ] `ProviderConfig.to_json()` の仕様確認と完全性検証
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

#### Refactor Phase: 品質改善と継続成功確認
- [ ] エラー時フォールバック（ルールベース要約へ）の動作確認
- [ ] ログ追加（API呼び出し情報、応答時間）
- [ ] テストを実行し、継続して成功することを確認

✅ **Phase Complete**

---

### Process 200: 長期改善 - キャッシュ・レート制限・フォールバック

**目的:** 要約結果のキャッシング、API rate limit対応、provider failover対応

#### Red Phase: 改善戦略設計
- [ ] キャッシュ戦略の設計（in-memory cache or persistent）
- [ ] レート制限検出メカニズム設計
- [ ] フォールバック戦略設計
- [ ] 設計ドキュメント完成を確認

✅ **Phase Complete**

#### Green Phase: 実装と成功確認
- [ ] キャッシュ実装（LRU cache 推奨）
- [ ] レート制限時のリトライ実装
- [ ] provider failover 実装
- [ ] テストを実行して成功することを確認

✅ **Phase Complete**

#### Refactor Phase: 品質改善と継続成功確認
- [ ] パフォーマンス測定
- [ ] テストを実行し、継続して成功することを確認
- [ ] ドキュメント更新

✅ **Phase Complete**

---

### Process 300: フィードバック収集 - 使用統計・エラー分析

**目的:** LLM要約の使用統計収集、エラーパターン分析、最適化の教訓保存

#### Red Phase: フィードバック収集設計

**Observe（観察）**
- [ ] 実装過程で発生した問題・課題を収集
- [ ] テスト結果から得られた知見を記録
- [ ] OpenRouter API 利用パターンを分析

**Orient（方向付け）**
- [ ] 収集した情報をカテゴリ別に分類
  - Technical: 技術的な知見・パターン（API呼び出しパターン、エラーハンドリング）
  - Process: プロセス改善に関する教訓（TDDの効果、テスト戦略）
  - Antipattern: 避けるべきパターン（同期API呼び出し、キャッシュなし）
  - Best Practice: 推奨パターン（非同期処理、エラーハンドリング戦略）
- [ ] 重要度（Critical/High/Medium/Low）を設定
- [ ] 分類完了を確認

✅ **Phase Complete**

#### Green Phase: 教訓・知見の永続化

**Decide（決心）**
- [ ] 保存すべき教訓・知見を選定
- [ ] 各項目の保存先を決定
  - Serena Memory: 組織的な知見（API統合パターン、エラーハンドリング戦略）
  - stigmergy/lessons: プロジェクト固有の教訓（digrag固有の設定・API使用法）
  - stigmergy/code-insights: コードパターン・実装知見

**Act（行動）**
- [ ] serena-v4 の mcp__serena__write_memory で教訓を永続化
- [ ] コードに関する知見を Markdown で記録
- [ ] 関連するコード箇所にコメントを追加（必要に応じて）
- [ ] 永続化完了を確認

✅ **Phase Complete**

#### Refactor Phase: フィードバック品質改善

**Feedback Loop**
- [ ] 保存した教訓の品質を検証
  - 再現可能性: 他のプロジェクトで適用可能か
  - 明確性: 内容が明確で理解しやすいか
  - 実用性: 実際に役立つ情報か
- [ ] 重複・矛盾する教訓を統合・整理
- [ ] メタ学習: OODA プロセス自体の改善点を記録

**Cross-Feedback**
- [ ] 他の Process（100, 200）との連携を確認
- [ ] 将来のミッションへの引き継ぎ事項を整理
- [ ] 検証完了を確認

✅ **Phase Complete**

---

## Management

### ブロッカー
- なし

### レッスン
- OpenRouterのプロバイダー設定は`provider`オブジェクトで柔軟にルーティング可能
- モデル指定は`provider/model`形式で統一

### フィードバックログ
| 日時 | 内容 |
|------|------|
| 2025-12-28 | 初期設計完了 |
| 2025-12-28 | 設定ファイル対応追加 |
| 2025-12-28 | LLM要約設定追加 |
| 2025-12-28 | OpenRouterプロバイダー設定追加 |

### 完了チェックリスト
- [ ] 全Processの実装完了
- [ ] 単体テスト作成・パス
- [ ] 統合テスト作成・パス
- [ ] OpenRouter API統合完了
- [ ] E2Eテスト作成・パス
- [ ] config.toml設定例をREADMEに追記
- [ ] コミット・PR作成

