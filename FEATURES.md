# digrag 機能ガイド

digrag に実装された主要機能の詳細説明と使用例。

---

## 目次

1. [コンテンツ抽出機能](#コンテンツ抽出機能)
2. [LLM 要約機能](#llm-要約機能)
3. [インクリメンタルビルド](#インクリメンタルビルド)
4. [MCP 統合](#mcp-統合)
5. [実装状況](#実装状況)

---

## コンテンツ抽出機能

### 概要

検索結果から返すコンテンツをどのように抽出するかを柔軟に制御できます。

### 抽出戦略（Extraction Strategy）

#### 1. Snippet（スニペット抽出）

最初の N 文字を抽出する最もシンプルな戦略。

**特徴**:
- 高速（テキスト処理なし）
- API コスト 0
- プレビュー用途に最適

**使用例**:
```bash
# MCP パラメータ
{
  "query": "機械学習",
  "extraction_mode": "snippet",
  "max_chars": 150   # デフォルト
}

# 結果例
{
  "snippet": "機械学習は統計学とコンピュータサイエンスの交差点に位置する分野です。データから自動的にパターンを学習し..."
}
```

**実装**:
```rust
pub enum ExtractionStrategy {
    Head(usize),  // N 文字抽出
}

// 使用例
let extractor = ContentExtractor::new(
    ExtractionStrategy::Head(150),
    TruncationConfig::default()
);
let result = extractor.extract(&document);
```

#### 2. ChangelogEntry（変更ログエントリ抽出）

`*` で始まる変更ログ形式のエントリを抽出する戦略。

**パターン**:
```markdown
* Title YYYY-MM-DD
  Content line 1
  Content line 2

* Another Title YYYY-MM-DD
  More content
```

**特徴**:
- 構造化テキストに最適
- エントリごとに分割可能
- 変更ログ・メモ形式ドキュメント向け

**使用例**:
```bash
# MCP パラメータ
{
  "query": "プロジェクト計画",
  "extraction_mode": "entry",
  "max_chars": 3000
}

# 入力ドキュメント
"* Project Planning 2024-01-15\n
  - Define scope\n
  - Create timeline\n
\n
* Project Review 2024-02-15\n
  - Checkpoint meeting\n
  - Budget review"

# 抽出結果
"* Project Planning 2024-01-15\n
  - Define scope\n
  - Create timeline\n
\n
* Project Review 2024-02-15\n
  - Checkpoint meeting\n
  - Budget review"
```

**実装**:
```rust
pub enum ExtractionStrategy {
    ChangelogEntry,  // 変更ログエントリ抽出
}

pub mod changelog {
    pub fn extract_changelog_entries(text: &str, max_chars: usize) -> String {
        // * YYYY-MM-DD パターンでエントリ分割
        // max_chars に達するまで抽出
    }
}
```

#### 3. Full（全文抽出）

ドキュメント全体を抽出します。

**特徴**:
- 完全な情報取得
- オプションで最大文字数制限

**使用例**:
```bash
# MCP パラメータ
{
  "query": "実装詳細",
  "extraction_mode": "full",
  "max_chars": 10000  # 10000 字で打ち切り
}
```

**実装**:
```rust
pub enum ExtractionStrategy {
    Full,  // 全文抽出
}
```

### 抽出パラメータ（TruncationConfig）

```rust
pub struct TruncationConfig {
    pub max_chars: Option<usize>,    // 最大文字数（デフォルト: 5000）
    pub max_lines: Option<usize>,    // 最大行数（オプション）
    pub max_sections: Option<usize>, // 最大セクション数（オプション）
}
```

### 抽出結果（ExtractedContent）

```rust
pub struct ExtractedContent {
    pub text: String,              // 抽出されたテキスト
    pub truncated: bool,           // 切り詰められたかどうか
    pub stats: ContentStats,       // 統計情報
}

pub struct ContentStats {
    pub total_chars: usize,        // 元の文字数
    pub total_lines: usize,        // 元の行数
    pub extracted_chars: usize,    // 抽出された文字数
}
```

### 使用パターン

#### パターン 1: 速度重視（プレビュー表示）

```rust
let extractor = ContentExtractor::new(
    ExtractionStrategy::Head(150),
    TruncationConfig { max_chars: Some(150), ..Default::default() }
);

let result = extractor.extract(&doc);
// 結果: "最初の 150 字..."
```

#### パターン 2: 構造重視（変更ログ）

```rust
let extractor = ContentExtractor::new(
    ExtractionStrategy::ChangelogEntry,
    TruncationConfig { max_chars: Some(5000), ..Default::default() }
);

let result = extractor.extract(&doc);
// 結果: "* エントリ 1\n  内容\n\n* エントリ 2\n  内容"
```

#### パターン 3: 完全性重視（詳細表示）

```rust
let extractor = ContentExtractor::new(
    ExtractionStrategy::Full,
    TruncationConfig { max_chars: Some(20000), ..Default::default() }
);

let result = extractor.extract(&doc);
// 結果: ドキュメント全体（20000 字まで）
```

---

## LLM 要約機能

### 概要

抽出されたコンテンツを LLM で要約します。

**2 つのモード**:
1. **ルールベース要約**: 統計情報のみ（API コスト 0）
2. **LLM ベース要約**: OpenRouter Chat Completions で生成（コスト あり）

### ルールベース要約

**機能**:
- コンテンツ統計情報の提供
- メタデータ（タイトル・タグ・日付）の抽出
- 構造解析（セクション数など）

**実装例**:

```rust
pub struct RuleBasedSummary {
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
    pub word_count: usize,
    pub section_count: usize,
    pub preview: String,  // 最初の 200 字
}

// 出力例
{
    "title": "Python 基礎トレーニング",
    "date": "2024-01-15",
    "tags": ["programming", "python"],
    "word_count": 3500,
    "section_count": 8,
    "preview": "Python は汎用プログラミング言語で可読性が高く..."
}
```

**用途**:
- クイックプレビュー
- ドキュメント一覧表示
- 高速応答が必要な場面

### LLM ベース要約

**機能**:
- OpenRouter Chat Completions API で高品質要約生成
- カスタムプロンプト対応
- プロバイダ選択機能

**API 設定**:

```rust
pub struct OpenRouterClient {
    api_key: String,
    base_url: String,  // https://openrouter.ai/api/v1
    model: String,     // デフォルト: meta-llama/llama-3.1-8b-instruct
}

pub struct ChatCompletionOptions {
    pub temperature: f32,           // 0.0-2.0 (デフォルト: 0.7)
    pub top_p: f32,                 // 0.0-1.0 (デフォルト: 1.0)
    pub max_tokens: usize,          // (デフォルト: 1000)
    pub provider: Option<ProviderConfig>,
}

pub struct ProviderConfig {
    pub order: Option<Vec<String>>,           // プロバイダ優先順位
    pub allow_fallbacks: bool,               // フォールバック許可
    pub only: Option<Vec<String>>,           // 指定プロバイダのみ
    pub ignore: Option<Vec<String>>,         // 除外プロバイダ
    pub sort: Option<String>,                // price|throughput でソート
    pub require_parameters: bool,            // パラメータ要件
}
```

**使用例**:

```bash
# MCP パラメータで LLM 要約を指定
{
  "query": "Python 基礎",
  "use_llm_summary": true,
  "include_summary": true,
  "extraction_mode": "full",
  "max_chars": 10000
}
```

**リクエスト例**:

```json
{
  "model": "meta-llama/llama-3.1-8b-instruct",
  "messages": [
    {
      "role": "user",
      "content": "以下のテキストを 200 字以内で日本語で要約してください：\n\n{extracted_content}"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 200,
  "providers": {
    "allow_fallbacks": true,
    "require_parameters": false
  }
}
```

**レスポンス例**:

```json
{
  "id": "...",
  "choices": [
    {
      "message": {
        "content": "Python は直感的で学習曲線が緩いプログラミング言語です。統計学やデータ分析、機械学習など様々な分野で活用されています。..."
      }
    }
  ],
  "usage": {
    "prompt_tokens": 450,
    "completion_tokens": 120,
    "total_tokens": 570
  }
}
```

**コスト例** (OpenRouter 2024-12-29 の相場):
```
モデル: meta-llama/llama-3.1-8b-instruct
Input:  $0.00003 / 1k tokens
Output: $0.00015 / 1k tokens

1 つの要約（平均 570 tokens）:
コスト = (450 * 0.00003 + 120 * 0.00015) / 1000 = $0.000015 ≈ $0.01 / 100 件
```

### 要約パイプライン

```
検索結果 (複数ドキュメント)
  ↓
┌─────────────────────────────────────────┐
│ 各ドキュメントについて                    │
├─────────────────────────────────────────┤
│ 1. ContentExtractor で抽出              │
│ 2. include_summary=true なら要約生成     │
│    ├─ use_llm_summary=false: ルールベース │
│    └─ use_llm_summary=true: LLM ベース   │
│ 3. 結果に含める                         │
└─────────────────────────────────────────┘
  ↓
最終レスポンス
{
  "results": [
    {
      "id": "...",
      "title": "...",
      "snippet": "...",
      "summary": {
        "type": "rule_based" | "llm_based",
        "content": "要約テキスト",
        "tokens_used": 570
      }
    },
    ...
  ]
}
```

---

## インクリメンタルビルド

### 概要

ドキュメント更新時に**変更分のみを処理**し、API コストを削減する機能。

### 実装メカニズム

#### ステップ 1: SHA256 ハッシュベース差分検出

```rust
pub fn compute_content_hash(title: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(b"\0");                    // セパレータ
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])                // 最初の 8 バイト = 16 hex 文字
}
```

**例**:
```
Document 1:
  title: "Python 基礎"
  text: "Python は..."
  → hash: "a3c5d2e1f7b9c4e6"

Document 2 (次回):
  title: "Python 基礎"
  text: "Python は... (修正版)"
  → hash: "b7e2f1d9c5a3e8f4"  (異なる)
```

#### ステップ 2: メタデータ保存・比較

```rust
pub struct IndexMetadata {
    pub version: u32,
    pub build_timestamp: DateTime<Utc>,
    pub content_hashes: HashMap<String, String>,  // doc_id → hash
    pub docstore_size: usize,
}

// metadata.json に保存
{
  "version": 1,
  "build_timestamp": "2024-12-20T10:30:00Z",
  "content_hashes": {
    "abc123": "a3c5d2e1f7b9c4e6",
    "def456": "e2b1f8c9a7d5e3c6",
    ...
  },
  "docstore_size": 640
}
```

#### ステップ 3: 差分分類

```rust
pub struct IncrementalDiff {
    pub added: Vec<Document>,        // 新規
    pub modified: Vec<Document>,     // 更新
    pub removed: Vec<String>,        // 削除
    pub unchanged: Vec<String>,      // 未変更
}

// 差分検出ロジック
pub fn detect(new_docs: &[Document], metadata: &IndexMetadata) -> IncrementalDiff {
    let mut diff = IncrementalDiff::default();

    for doc in new_docs {
        let current_hash = compute_content_hash(&doc.title, &doc.text);
        if let Some(prev_hash) = metadata.content_hashes.get(&doc.id) {
            if current_hash == prev_hash {
                diff.unchanged.push(doc.id.clone());
            } else {
                diff.modified.push(doc.clone());
            }
        } else {
            diff.added.push(doc.clone());
        }
    }

    // 削除検出
    for (doc_id, _) in &metadata.content_hashes {
        if !new_docs.iter().any(|d| &d.id == doc_id) {
            diff.removed.push(doc_id.clone());
        }
    }

    diff
}
```

#### ステップ 4: 選択的処理

```
Incremental build command:
digrag build --input ~/notes --with-embeddings --incremental

┌──────────────────────────────────────────────┐
│ Added (5 docs)     ← BM25 + Embeddings 処理 │
├──────────────────────────────────────────────┤
│ Modified (2 docs)  ← BM25 + Embeddings 処理 │
├──────────────────────────────────────────────┤
│ Removed (1 doc)    ← Docstore から削除      │
├──────────────────────────────────────────────┤
│ Unchanged (632)    ← スキップ (インデックス  │
│                      にはすでに存在)          │
└──────────────────────────────────────────────┘

実際に処理: 5 + 2 = 7 docs
全文書数: 640 docs
処理率: 7/640 = 1.09%

Embeddings API 呼び出し: 7 回（従来は 640 回）
```

### コスト削減の実例

#### シナリオ: 企業ナレッジベース

```
運用期間: 12 ヶ月（毎週更新）
総文書数: 640 文書
週次変更率: 約 1-2% (平均 7 文書)

従来型（毎回フル構築）:
  52 週 × 640 文書 = 33,280 API 呼び出し
  コスト: 33,280 × $0.00003 = $1.00 (OpenRouter 相場)

インクリメンタル型:
  52 週 × 7 文書 = 364 API 呼び出し
  コスト: 364 × $0.00003 = $0.011

削減率: (33,280 - 364) / 33,280 = 98.9%
削減額: $0.99/year（小規模だが、大規模では数万ドル削減可能）
```

### 使用方法

#### 初回構築（フルビルド）

```bash
# --incremental フラグを付けても初回は全文書処理
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental

# 出力
Loading documents from ~/notes...
Loaded 640 documents

Using incremental build mode
Note: First run will process all documents to establish baseline

Building BM25 index... ✓
Generating embeddings (640 documents)... [████████████████] 100%
Building vector index... ✓
Saving metadata for future incremental builds...

✓ bm25_index.json
✓ faiss_index.json
✓ docstore.json
✓ metadata.json (baseline established)
```

#### 2 回目以降の更新

```bash
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental

# 出力
Loading documents from ~/notes...
Loaded 641 documents

Using incremental build mode
Loaded 640 documents total (from previous build)

Incremental build summary:
  Added: 1 documents
  Modified: 6 documents
  Removed: 0 documents
  Unchanged: 633 documents
  Embeddings needed: 7

Building BM25 index... ✓
Generating embeddings (7 documents)... [████████] 100%
Building vector index... ✓
Updating metadata...

✓ bm25_index.json
✓ faiss_index.json
✓ docstore.json
✓ metadata.json (updated)
```

#### 全体再構築（メタデータ破損時など）

```bash
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental --force

# --force で前回のメタデータを無視して全文書処理
```

---

## MCP 統合

### 概要

digrag を MCP（Model Context Protocol）サーバーとして実行し、Claude Code や Claude Desktop から直接利用できます。

### アーキテクチャ

```
Claude Code / Claude Desktop
        ↓ (JSON-RPC over stdin/stdout)
┌────────────────────────────────────┐
│     DigragMcpServer                │
│  (src/main.rs の MCP ハンドラー)    │
└────────┬───────────────────────────┘
         │ Rust の rmcp マクロで実装
         ↓
┌────────────────────────────────────┐
│  提供ツール（3 つ）               │
├────────────────────────────────────┤
│ 1. query_memos                     │
│    → Searcher (BM25/Semantic/Hybrid)│
│    → ContentExtractor              │
│    → Summarizer (optional)         │
│                                    │
│ 2. list_tags                       │
│    → Docstore.list_tags()          │
│                                    │
│ 3. get_recent_memos                │
│    → Docstore.get_recent()         │
└────────────────────────────────────┘
```

### 3 つの MCP ツール

#### Tool 1: query_memos

ドキュメントを複数の検索モードで検索。

**パラメータ定義**:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryMemosParams {
    /// 検索クエリ（必須）
    pub query: String,

    /// 返す結果件数（デフォルト: 10）
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// オプション: タグフィルタ
    pub tag_filter: Option<String>,

    /// 検索モード: "bm25" | "semantic" | "hybrid"（デフォルト: "bm25"）
    #[serde(default = "default_mode")]
    pub mode: String,

    /// コンテンツ抽出戦略: "snippet" | "entry" | "full"（デフォルト: "snippet"）
    #[serde(default = "default_extraction_mode")]
    pub extraction_mode: String,

    /// 最大抽出文字数（デフォルト: 5000）
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,

    /// 要約を含める（デフォルト: true）
    #[serde(default = "default_true")]
    pub include_summary: bool,

    /// 生テキストを含める（デフォルト: true）
    #[serde(default = "default_true")]
    pub include_raw: bool,

    /// LLM ベース要約を使用（デフォルト: false、コスト増）
    #[serde(default)]
    pub use_llm_summary: bool,
}
```

**レスポンス型**:

```rust
pub struct QueryMemosResponse {
    pub results: Vec<MemoResult>,
    pub total: usize,
}

pub struct MemoResult {
    pub id: String,
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
    pub snippet: String,
    pub score: f32,
    // オプション（パラメータで制御）
    pub summary: Option<Summary>,
    pub raw_content: Option<String>,
}

pub struct Summary {
    pub summary_type: String,     // "rule_based" | "llm_based"
    pub content: String,
    pub tokens_used: Option<usize>,
}
```

**実装フロー** (`src/main.rs`):

```rust
#[tool]
pub async fn query_memos(params: QueryMemosParams) -> CallToolResult {
    // 1. パラメータ検証
    let search_config = SearchConfig {
        mode: match params.mode.as_str() {
            "semantic" => SearchMode::Semantic,
            "hybrid" => SearchMode::Hybrid,
            _ => SearchMode::BM25,
        },
        top_k: params.top_k,
        ..Default::default()
    };

    // 2. 検索実行
    let results = searcher.search(&params.query, &search_config)?;

    // 3. コンテンツ抽出
    let extractor = ContentExtractor::new(
        match params.extraction_mode.as_str() {
            "entry" => ExtractionStrategy::ChangelogEntry,
            "full" => ExtractionStrategy::Full,
            _ => ExtractionStrategy::Head(150),
        },
        TruncationConfig { max_chars: Some(params.max_chars), ..Default::default() },
    );

    // 4. 要約生成（オプション）
    let mut responses = vec![];
    for result in results {
        let extracted = extractor.extract(&result.doc)?;

        let summary = if params.include_summary {
            if params.use_llm_summary {
                // LLM ベース要約（API コスト発生）
                Some(summarizer.summarize_llm(&extracted.text).await?)
            } else {
                // ルールベース要約
                Some(summarizer.summarize_rule(&result.doc))
            }
        } else {
            None
        };

        responses.push(MemoResult {
            id: result.id,
            title: result.title,
            date: result.date,
            tags: result.tags,
            snippet: extracted.text,
            score: result.score,
            summary,
            raw_content: if params.include_raw { Some(result.text) } else { None },
        });
    }

    // 5. レスポンス返却
    CallToolResult {
        content: vec![Content::Text {
            text: serde_json::to_string(&QueryMemosResponse {
                results: responses,
                total: responses.len(),
            })?,
        }],
        is_error: false,
    }
}
```

#### Tool 2: list_tags

インデックス内のすべてのタグをリストアップ。

**パラメータ**: なし

**レスポンス型**:

```rust
pub struct ListTagsResponse {
    pub tags: Vec<TagInfo>,
}

pub struct TagInfo {
    pub name: String,
    pub count: usize,
}
```

**実装**:

```rust
#[tool]
pub async fn list_tags() -> CallToolResult {
    let tags = docstore.list_tags();
    let response = ListTagsResponse { tags };

    CallToolResult {
        content: vec![Content::Text {
            text: serde_json::to_string(&response)?,
        }],
        is_error: false,
    }
}
```

#### Tool 3: get_recent_memos

最近更新されたドキュメントを取得。

**パラメータ**:

```rust
pub struct GetRecentMemosParams {
    /// 取得件数（デフォルト: 10）
    #[serde(default = "default_limit")]
    pub limit: usize,
}
```

**レスポンス型**:

```rust
pub struct GetRecentMemosResponse {
    pub memos: Vec<MemoResult>,
}
```

**実装**:

```rust
#[tool]
pub async fn get_recent_memos(params: GetRecentMemosParams) -> CallToolResult {
    let memos = docstore.get_recent(params.limit);
    let response = GetRecentMemosResponse { memos };

    CallToolResult {
        content: vec![Content::Text {
            text: serde_json::to_string(&response)?,
        }],
        is_error: false,
    }
}
```

### Claude 内での使用フロー

```
Claude へのプロンプト:
"my_notes のインデックスで、'プロジェクト管理' について
ハイブリッド検索してください。最新の 3 つのドキュメントも教えてください。"

↓

Claude が以下のツール呼び出しを生成:

Call 1: digrag:query_memos
{
  "query": "プロジェクト管理",
  "mode": "hybrid",
  "top_k": 10,
  "extraction_mode": "snippet",
  "include_summary": true,
  "use_llm_summary": false
}

Call 2: digrag:get_recent_memos
{
  "limit": 3
}

↓

digrag MCP サーバーが処理:

1. query_memos:
   - Searcher で "プロジェクト管理" をハイブリッド検索
   - 結果 10 件に対して ContentExtractor で snippet 抽出
   - Summarizer で ルールベース要約生成

2. get_recent_memos:
   - Docstore から最新 3 ドキュメント取得

↓

Claude に結果返却:

{
  "results": [
    {
      "id": "abc123",
      "title": "プロジェクト管理手法",
      "date": "2024-01-15",
      "tags": ["management", "agile"],
      "snippet": "プロジェクト管理には様々な手法があります...",
      "score": 0.92,
      "summary": {
        "summary_type": "rule_based",
        "content": "Agile, Scrum, Waterfall などのプロジェクト管理手法を比較..."
      }
    },
    ...
  ],
  "total": 10
}

{
  "memos": [
    {
      "id": "xyz789",
      "title": "2024-12-20 進捗レポート",
      "date": "2024-12-20",
      "tags": ["report"],
      "snippet": "今週のハイライト: ...",
      "score": 1.0
    },
    ...
  ]
}

↓

Claude が結果を解釈・統合して回答
```

---

## 実装状況

### ✅ 実装済み機能

| 機能 | 状態 | 説明 |
|---|---|---|
| **CLI: init** | ✅ | 設定ファイル初期化 |
| **CLI: build** | ✅ | インデックス構築 |
| **CLI: search** | ✅ | CLI 検索（テスト用） |
| **CLI: serve** | ✅ | MCP サーバー起動 |
| **BM25 検索** | ✅ | キーワード検索 |
| **セマンティック検索** | ✅ | OpenRouter Embeddings 統合 |
| **ハイブリッド検索** | ✅ | RRF による結果融合 |
| **日本語トークン化** | ✅ | Lindera IPADIC 対応 |
| **コンテンツ抽出** | ✅ | Snippet / Entry / Full 3 戦略 |
| **ルールベース要約** | ✅ | 統計情報の提供 |
| **LLM ベース要約** | ✅ | OpenRouter Chat API 統合 |
| **インクリメンタルビルド** | ✅ | SHA256 ハッシュベース差分検出 |
| **MCP サーバー** | ✅ | 3 つのツール公開 |
| **query_memos** | ✅ | 検索ツール |
| **list_tags** | ✅ | タグ一覧ツール |
| **get_recent_memos** | ✅ | 最新文書取得ツール |

### 🔄 計画中の機能

| 機能 | 説明 | 優先度 |
|---|---|---|
| **Web UI** | ブラウザベース検索インターフェース | 中 |
| **REST API** | HTTP API エンドポイント | 中 |
| **gRPC Server** | 高性能 RPC インターフェース | 低 |
| **キャッシング** | クエリキャッシュ機能拡張 | 中 |
| **マルチ言語** | 日本語以外の言語対応の強化 | 低 |
| **プラグイン** | カスタム抽出戦略の追加機能 | 低 |

---

**最終更新**: 2024-12-29
