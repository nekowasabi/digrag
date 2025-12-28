# digrag 完全ガイド

**digrag**: ポータブル RAG（Retrieval-Augmented Generation）検索エンジン

高性能で拡張性の高いテキスト検索エンジンで、メモ、ドキュメント、ナレッジベースに対して BM25 キーワード検索、セマンティック（ベクトル）検索、ハイブリッド検索を提供します。Claude の MCP（Model Context Protocol）サーバーとして統合でき、Claude Code や Claude Desktop から直接検索できます。

---

## Table of Contents

1. [アーキテクチャ概要](#アーキテクチャ概要)
2. [セットアップ](#セットアップ)
3. [CLI 使い方ガイド](#cli-使い方ガイド)
4. [MCP サーバー統合](#mcp-サーバー統合)
5. [機能詳細](#機能詳細)
6. [実践例](#実践例)
7. [トラブルシューティング](#トラブルシューティング)

---

## アーキテクチャ概要

### プロジェクト構成図

```
digrag
├── CLI コマンド層
│   ├── init       → 設定ファイル初期化
│   ├── build      → インデックス構築（BM25 + Optional Vector）
│   ├── search     → CLI 検索（テスト・手動検索用）
│   └── serve      → MCP サーバー起動
│
├── インデックス層
│   ├── BM25Index          → キーワード検索用 Inverted Index
│   ├── VectorIndex        → セマンティック検索用 FAISS
│   ├── Docstore          → メタデータ・ドキュメント本体
│   └── IndexMetadata     → インクリメンタルビルド用メタデータ
│
├── 検索エンジン層
│   ├── Searcher          → 検索統合インターフェース
│   ├── BM25Searcher      → キーワード検索実装
│   ├── VectorSearcher    → セマンティック検索実装
│   └── ReciprocalRankFusion → ハイブリッド結果融合
│
├── 抽出・要約層
│   ├── ContentExtractor  → コンテンツ抽出（複数戦略）
│   ├── Summarizer       → ルールベース/LLM ベース要約
│   └── OpenRouterClient → 外部 LLM API（要約用）
│
├── データ処理層
│   ├── ChangelogLoader  → Markdown / JSONL ファイル読み込み
│   ├── Tokenizer       → 日本語トークン化（Lindera IPADIC）
│   ├── OpenRouterEmbedding → OpenRouter Embeddings API
│   └── QueryRewriter    → LLM ベースクエリ最適化
│
├── MCP サーバー層
│   └── DigragMcpServer  → Claude 統合（3 つのツール公開）
│
└── 設定層
    ├── AppConfig        → アプリケーション設定管理
    ├── SearchConfig     → 検索パラメータ設定
    └── PathResolver     → クロスプラットフォーム対応
```

### データフロー

```
┌─────────────────────────────────────────────────────────────┐
│                    入力： テキストファイル群                    │
│              （Markdown, JSONL, Plain Text）                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ↓
┌─────────────────────────────────────────────────────────────┐
│   Phase 1: ドキュメント読み込み & トークン化                   │
│   • ChangelogLoader: ファイル読み込み                         │
│   • Tokenizer: 日本語トークン化（Lindera IPADIC）             │
└────────────────────────┬────────────────────────────────────┘
                         │
                    ┌────┴────┐
                    ↓         ↓
            ┌──────────────┐  ┌──────────────────┐
            │ BM25 Index   │  │ Vector Index     │
            │   構築       │  │   構築（Optional）│
            │              │  │ OpenRouter API   │
            └──────┬───────┘  └────────┬─────────┘
                   │                   │
                   └────────┬──────────┘
                            ↓
        ┌──────────────────────────────────────┐
        │     インデックスファイル出力              │
        │  • bm25_index.json                   │
        │  • faiss_index.json（Optional）      │
        │  • docstore.json                     │
        │  • metadata.json（増分ビルド用）        │
        └──────────────┬───────────────────────┘
                       │
           ┌───────────┴───────────┐
           ↓                       ↓
    ┌────────────────┐     ┌──────────────────┐
    │  CLI Search    │     │  MCP Server      │
    │  • bm25        │     │  (Claude 統合)   │
    │  • semantic    │     │  • query_memos   │
    │  • hybrid      │     │  • list_tags     │
    └────────────────┘     │  • get_recent    │
                           └──────────────────┘

クエリ処理フロー：
  クエリ入力 → クエリ書き換え（Optional LLM） →
  BM25 検索 / Vector 検索 → RRF 融合（Hybrid） →
  コンテンツ抽出 → 要約（Optional LLM） → 結果返却
```

### 主要コンポーネント表

| コンポーネント | 責務 | 主要ファイル |
|---|---|---|
| **ChangelogLoader** | Markdown/JSONL ファイル読み込み、メタデータ抽出 | `loader/changelog.rs`, `loader/document.rs` |
| **BM25Index** | キーワードベース検索インデックス構築・検索 | `index/bm25.rs` |
| **VectorIndex** | FAISS ベースセマンティック検索 | `index/vector.rs` |
| **Docstore** | ドキュメント本体・メタデータ保存 | `index/docstore.rs` |
| **Searcher** | BM25/Vector/Hybrid 検索の統合インターフェース | `search/searcher.rs` |
| **ReciprocalRankFusion** | BM25 と Vector 結果の統合ランキング | `search/fusion.rs` |
| **ContentExtractor** | 複数戦略によるコンテンツ抽出 | `extract/mod.rs` |
| **Summarizer** | ルールベース・LLM ベース要約 | `extract/summarizer.rs` |
| **OpenRouterEmbedding** | OpenRouter Embeddings API クライアント | `embedding/openrouter.rs` |
| **OpenRouterClient** | OpenRouter Chat Completions API（LLM 機能用） | `extract/openrouter_client.rs` |
| **DigragMcpServer** | MCP サーバー実装（Claude 統合） | `mcp/server.rs`, `main.rs` |
| **IndexBuilder** | インクリメンタル対応のインデックス構築パイプライン | `index/builder.rs` |
| **IncrementalDiff** | SHA256 ハッシュベース差分検出 | `index/diff.rs` |

---

## セットアップ

### インストール方法

#### 1. バイナリダウンロード（推奨）

```bash
# GitHub Releases から最新バイナリをダウンロード
# macOS (Apple Silicon)
curl -LO https://github.com/takets/digrag/releases/latest/download/digrag-aarch64-apple-darwin.tar.gz
tar xzf digrag-aarch64-apple-darwin.tar.gz
sudo mv digrag /usr/local/bin/

# Linux (x86_64)
curl -LO https://github.com/takets/digrag/releases/latest/download/digrag-x86_64-unknown-linux-gnu.tar.gz
tar xzf digrag-x86_64-unknown-linux-gnu.tar.gz
sudo mv digrag /usr/local/bin/

# Windows (x86_64)
# https://github.com/takets/digrag/releases/latest/download/digrag-x86_64-pc-windows-msvc.zip

digrag --version
```

#### 2. cargo install（Rust 1.70+ 必須）

```bash
cargo install digrag
digrag --version
```

#### 3. ソースからビルド

```bash
git clone https://github.com/takets/digrag.git
cd digrag
make build-release
./target/release/digrag --version
```

### 環境設定

#### API キー設定（セマンティック検索使用時）

```bash
# OpenRouter API キー取得
# https://openrouter.ai から新規登録・API キー取得

# 環境変数設定
export OPENROUTER_API_KEY="sk-or-v1-..."

# 永続設定（~/.zshrc または ~/.bashrc に追加）
echo 'export OPENROUTER_API_KEY="sk-or-v1-..."' >> ~/.zshrc
source ~/.zshrc
```

#### 設定ファイル初期化

```bash
digrag init
```

出力される設定ファイル場所：
- **Linux/macOS**: `~/.config/digrag/config.toml`
- **Windows**: `%APPDATA%\digrag\config.toml`

設定ファイル例：
```toml
[search]
default_mode = "bm25"  # bm25, semantic, hybrid
default_top_k = 10

[extraction]
default_extraction_mode = "snippet"  # snippet, entry, full
max_chars = 5000

[embedding]
# OpenRouter 埋め込みモデル設定
model = "mixedbread-ai/mxbai-embed-large-v1"
batch_size = 50

[rewriter]
# LLM ベースクエリ書き換え
enable = false
model = "meta-llama/llama-3.1-8b-instruct"
```

---

## CLI 使い方ガイド

### 1. インデックス構築（build）

インデックスの構築は**1 回目の初期化**と**以降の更新**で異なります。

#### 基本的な使い方

```bash
# BM25 のみ（高速、API キー不要）
digrag build \
  --input ~/notes \
  --output ~/.digrag/index

# セマンティック検索対応（Embeddings 生成）
digrag build \
  --input ~/notes \
  --output ~/.digrag/index \
  --with-embeddings

# インクリメンタルビルド（変更分のみ処理、2 回目以降推奨）
digrag build \
  --input ~/notes \
  --output ~/.digrag/index \
  --with-embeddings \
  --incremental
```

#### パラメータ詳細

| パラメータ | 短縮 | 説明 | デフォルト |
|---|---|---|---|
| `--input <PATH>` | `-i` | ソースファイル/ディレクトリ（必須） | - |
| `--output <PATH>` | `-o` | インデックス出力ディレクトリ | `.rag` |
| `--with-embeddings` | - | OpenRouter Embeddings で Vector Index 生成 | false |
| `--skip-embeddings` | - | Embeddings 生成をスキップ | false |
| `--incremental` | - | インクリメンタルビルド（変更分のみ処理） | false |
| `--force` | - | インクリメンタル時に全体再構築 | false |

#### 実行例と出力

```bash
$ digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental

Loading documents from ~/notes...
Loaded 640 documents

Using incremental build mode
Loaded 640 documents total

Incremental build summary:
  Added: 5 documents
  Modified: 2 documents
  Removed: 1 documents
  Unchanged: 632 documents
  Embeddings needed: 7

Building BM25 index... ✓
Generating embeddings (7 documents)... ✓
Building vector index... ✓
Saving indices to ~/.digrag/index...
✓ bm25_index.json (640 docs)
✓ faiss_index.json (640 vectors)
✓ docstore.json (640 docs)
✓ metadata.json (incremental tracking)

Index built successfully! (35.2s)
```

#### インクリメンタルビルドの動作

- **1 回目**: `--incremental` フラグを使用してもフル構築
- **2 回目以降**: SHA256 ハッシュで変更検出、差分のみ処理
- **コスト削減**: 640 文書中 7 文書の差分処理で Embeddings API コスト約 99% 削減

### 2. 検索（search）

CLI 検索はテスト・手動検索用です。本運用では MCP サーバー経由が推奨です。

#### 基本的な使い方

```bash
# BM25 キーワード検索（デフォルト）
digrag search "会議メモ" --index-dir ~/.digrag/index

# セマンティック検索（意味ベース）
digrag search "プロジェクト計画" --index-dir ~/.digrag/index --mode semantic

# ハイブリッド検索（BM25 + Semantic の統合）
digrag search "アイデア" --index-dir ~/.digrag/index --mode hybrid

# タグフィルタ付き検索
digrag search "実装" --index-dir ~/.digrag/index --tag "技術"

# 結果件数指定
digrag search "デザイン" --index-dir ~/.digrag/index --top-k 5
```

#### パラメータ詳細

| パラメータ | 短縮 | 説明 | デフォルト |
|---|---|---|---|
| `<QUERY>` | - | 検索クエリ（必須） | - |
| `--index-dir <PATH>` | `-i` | インデックスディレクトリ | `.rag` |
| `--mode <MODE>` | `-m` | `bm25`, `semantic`, `hybrid` | `bm25` |
| `--top-k <NUM>` | `-k` | 返す結果件数 | `10` |
| `--tag <TAG>` | `-t` | タグフィルタ | - |
| `--verbose` | `-v` | 詳細ログ出力 | false |

#### 実行例と出力

```bash
$ digrag search "Python チュートリアル" --index-dir ~/.digrag/index --mode hybrid --top-k 5

BM25 search results (score: sum of IDF weights):
1. [0.89] Python 基礎トレーニング
   Date: 2024-01-15, Tags: [programming, python]
   Snippet: Python は汎用プログラミング言語で、可読性が高く...

2. [0.76] 機械学習入門
   Date: 2024-02-20, Tags: [ml, python]
   Snippet: Python の scikit-learn ライブラリを使って...

Semantic search results (cosine similarity):
1. [0.92] Python パフォーマンス最適化
   Date: 2024-03-10, Tags: [performance, python]
   Snippet: Python のコード最適化テクニック、メモリ管理...

2. [0.87] 関数型プログラミング入門
   Date: 2024-01-20, Tags: [functional, programming]
   Snippet: 関数型プログラミングの基本概念...

Hybrid results (RRF fusion):
1. [RRF: 1.67] Python 基礎トレーニング (BM25: #1, Semantic: #4)
2. [RRF: 1.52] Python パフォーマンス最適化 (BM25: #3, Semantic: #1)
3. [RRF: 1.43] 機械学習入門 (BM25: #2, Semantic: #5)
```

#### 検索モードの選択ガイド

| モード | 最適な用途 | 処理時間 | API コスト |
|---|---|---|---|
| **BM25** | キーワード検索、正確なマッチ | ~30 μs | 無料 |
| **Semantic** | 概念検索、言い換え対応 | ~3 ns（ベクトル検索） | ー（事前生成） |
| **Hybrid** | バランス型、精度重視 | ~34 μs | ー（事前生成） |

### 3. MCP サーバー起動（serve）

```bash
# ローカルテスト用（フォアグラウンド実行）
digrag serve --index-dir ~/.digrag/index

# 詳細ログ表示
digrag serve --index-dir ~/.digrag/index --verbose

# カスタムポート（オプション、設定ファイルで管理推奨）
digrag serve --index-dir ~/.digrag/index
```

---

## MCP サーバー統合

### Claude Code への統合

Claude Code の MCP 設定に digrag サーバーを追加します。

#### 設定方法

1. **設定ファイル編集**

   Claude Code のプロジェクトディレクトリに `.mcp.json` を作成：

   ```json
   {
     "mcpServers": {
       "digrag": {
         "command": "digrag",
         "args": ["serve", "--index-dir", "~/.digrag/index"],
         "env": {
           "OPENROUTER_API_KEY": "sk-or-v1-..."
         }
       }
     }
   }
   ```

2. **Claude Code の MCP 設定画面から追加**

   Claude Code → Settings → MCP Servers → Add new server
   - Server name: `digrag`
   - Command: `digrag`
   - Arguments: `serve --index-dir ~/.digrag/index`
   - Environment: `OPENROUTER_API_KEY=sk-or-v1-...`

### Claude Desktop への統合

1. **設定ファイル編集**

   `~/.claude/claude_desktop_config.json` を編集（ファイルが無い場合は新規作成）：

   ```json
   {
     "mcpServers": {
       "digrag": {
         "command": "/usr/local/bin/digrag",
         "args": ["serve", "--index-dir", "/Users/yourname/.digrag/index"],
         "env": {
           "OPENROUTER_API_KEY": "sk-or-v1-..."
         }
       }
     }
   }
   ```

   **パス設定のポイント**:
   - Linux/macOS: 絶対パス使用 `/usr/local/bin/digrag`
   - Windows: `C:\\Program Files\\digrag.exe` など
   - インデックスパスも絶対パス指定

2. **Claude Desktop 再起動**

   設定変更後、Claude Desktop を完全に終了・再起動してください。

### 公開される MCP ツール

digrag MCP サーバーは以下の 3 つのツールを Claude に公開します。

#### Tool 1: query_memos

ドキュメントを複数の検索モードで検索します。

**パラメータ**:

```json
{
  "query": "検索キーワード（必須）",
  "top_k": 10,                              // 結果件数（デフォルト: 10）
  "mode": "bm25",                           // bm25 | semantic | hybrid
  "tag_filter": "python",                   // タグフィルタ（オプション）
  "extraction_mode": "snippet",             // snippet | entry | full
  "max_chars": 5000,                        // 最大抽出文字数
  "include_summary": true,                  // 要約を含める
  "include_raw": true,                      // 生テキストを含める
  "use_llm_summary": false                  // LLM ベース要約（コスト増）
}
```

**レスポンス例**:

```json
{
  "results": [
    {
      "id": "abc123xyz",
      "title": "Python 基礎トレーニング",
      "date": "2024-01-15",
      "tags": ["programming", "python"],
      "snippet": "Python は汎用プログラミング言語で...",
      "score": 0.89
    },
    {
      "id": "def456uvw",
      "title": "機械学習入門",
      "date": "2024-02-20",
      "tags": ["ml", "python"],
      "snippet": "scikit-learn ライブラリを使って...",
      "score": 0.76
    }
  ],
  "total": 2
}
```

#### Tool 2: list_tags

インデックス内の全タグと各タグの文書数をリスト。

**パラメータ**: なし

**レスポンス例**:

```json
{
  "tags": [
    { "name": "programming", "count": 45 },
    { "name": "python", "count": 23 },
    { "name": "ml", "count": 18 },
    { "name": "javascript", "count": 15 },
    { "name": "architecture", "count": 12 }
  ]
}
```

#### Tool 3: get_recent_memos

最近更新されたドキュメントを取得します。

**パラメータ**:

```json
{
  "limit": 10  // 取得件数（デフォルト: 10）
}
```

**レスポンス例**:

```json
{
  "memos": [
    {
      "id": "abc123xyz",
      "title": "2024-12-20 進捗レポート",
      "date": "2024-12-20",
      "tags": ["report", "weekly"],
      "snippet": "今週の主な成果: インデックス機能...",
      "score": 1.0
    },
    {
      "id": "def456uvw",
      "title": "2024-12-19 チーム会議",
      "date": "2024-12-19",
      "tags": ["meeting", "team"],
      "snippet": "参加者: Alice, Bob, Charlie...",
      "score": 1.0
    }
  ]
}
```

### Claude 内での使用例

Claude Code または Claude Desktop で digrag ツールを使用：

```
Claude へのプロンプト例：

"my_notes のインデックスで 'データベース設計' というキーワードをハイブリッド検索してください。
最新のドキュメント 5 件も教えてください。"

→ Claude がツール呼び出し
digrag:query_memos {
  "query": "データベース設計",
  "mode": "hybrid",
  "top_k": 10
}

digrag:get_recent_memos {
  "limit": 5
}

→ 結果の統合と解釈をClaude が実行
```

---

## 機能詳細

### 検索機能

#### 1. BM25 キーワード検索

**仕組み**:
- **逆索引（Inverted Index）** でトークンを管理
- **TF-IDF + BM25** アルゴリズムでランキング
- クエリの各キーワードに対して文書の関連度を計算

**特徴**:
- 正確なキーワードマッチ
- 日本語形態素解析対応
- API キー不要、高速（~30 μs/クエリ）

**最適な用途**:
- 特定のキーワードを含む文書検索
- 技術用語・人名検索
- データベース検索

**例**:
```bash
digrag search "Docker" --mode bm25
# → "Docker" を含む全文書をスコア順に返却
```

#### 2. セマンティック検索

**仕組み**:
- クエリと各文書を **ベクトル埋め込み** に変換
- ベクトル空間上で **コサイン類似度** で検索
- FAISS（Facebook AI Similarity Search）でインデックス管理

**API**: OpenRouter Embeddings
- モデル: `mixedbread-ai/mxbai-embed-large-v1`
- 次元数: 1024
- 言語対応: 多言語（日本語含む）

**特徴**:
- 概念や意味での検索（言い換え・シノニム対応）
- 長文クエリに強い
- 前処理時間のみ（検索は ~3 ns）

**最適な用途**:
- 意味検索（「プロジェクト管理方法」で「Agile」を見つける）
- 言い換え検索
- 長文クエリ

**例**:
```bash
digrag search "プロジェクト管理の方法論" --mode semantic
# → Agile, Scrum, Waterfall など概念的に関連したドキュメントを返却
```

#### 3. ハイブリッド検索（RRF: Reciprocal Rank Fusion）

**仕組み**:
```
RRF スコア = 1/（k + BM25_rank） + 1/（k + Semantic_rank）
デフォルト k = 60
```

- BM25 検索と Semantic 検索を別途実行
- 各結果のランクを正規化して融合
- 両検索の長所を活かした精度向上

**特徴**:
- キーワード精度 + 意味理解
- 信頼度が高い結果がランク上位に
- 最も推奨される検索モード

**最適な用途**:
- 本運用（精度重視）
- 複合的な検索要求

**例**:
```bash
digrag search "Python での機械学習実装" --mode hybrid
# → BM25 で "Python" "機械学習" "実装" を含む文書 +
#   Semantic で意味的に関連した文書を統合
```

### コンテンツ抽出機能

ドキュメント本体をどう抽出するか複数の戦略に対応します。

#### 抽出戦略

| 戦略 | 説明 | 用途 |
|---|---|---|
| **Snippet** | 最初の N 文字を抽出（デフォルト 150 字） | プレビュー用 |
| **ChangelogEntry** | `*` で始まる変更ログエントリを抽出 | 変更ログの構造化抽出 |
| **Full** | 全文抽出（オプションで最大文字数制限） | 詳細表示用 |

#### 使用例（MCP パラメータ）

```json
{
  "query": "Python",
  "extraction_mode": "snippet",     // デフォルト、最初 150 字
  "max_chars": 5000                 // 最大 5000 字まで
}
```

### 要約機能（実装予定）

**ルールベース要約**:
- コンテンツ統計情報（文字数、行数、段落数）を提供
- API コスト 0

**LLM ベース要約**:
- OpenRouter Chat Completions API で要約生成
- 高精度だがコスト増

使用例：
```json
{
  "query": "チューリング賞",
  "include_summary": true,
  "use_llm_summary": false          // ルールベース
}
```

### インクリメンタルビルド

**何が問題か**:
- 640 文書中 7 文書変更した場合、全 640 文書の Embeddings を再生成→API コスト増加

**解決策**: SHA256 ハッシュベース差分検出

```
1. 各文書の（タイトル + テキスト）の SHA256 ハッシュを計算
2. metadata.json に前回のハッシュを保存
3. 再構築時に新ハッシュと比較
4. 変更あり（追加・更新・削除）の文書のみ処理
```

**実際の効果**:
```
Total documents: 640
  Added: 5
  Modified: 2
  Removed: 1
  Unchanged: 632

Embeddings needed: 7（640 のうち）
→ API コスト 99% 削減
```

**使用方法**:
```bash
# 初回はフルビルド
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings

# 2 回目以降はインクリメンタル
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental

# 全体再構築が必要な場合（前回メタデータが破損など）
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental --force
```

---

## 実践例

### ユースケース 1: 個人メモの検索

**シナリオ**: Markdown 形式の個人メモ 200 個を検索

```bash
# Step 1: インデックス構築（初回）
digrag build --input ~/Documents/my_notes --output ~/.digrag/notes_index --with-embeddings

# Step 2: CLI で検索テスト
digrag search "Python セットアップ" --index-dir ~/.digrag/notes_index --top-k 5

# Step 3: MCP サーバー設定（Claude Code）
# .mcp.json に追加
{
  "digrag_notes": {
    "command": "digrag",
    "args": ["serve", "--index-dir", "~/.digrag/notes_index"]
  }
}

# Step 4: Claude Code から利用
# Claude へ: "my_notes から 'Rust 言語' に関する資料を探してください"
→ claude が digrag:query_memos を呼び出し
```

### ユースケース 2: 企業ナレッジベースの検索

**シナリオ**: 5000 文書の企業ドキュメント、セマンティック検索対応

```bash
# 初期構築（コスト最適化）
digrag build \
  --input /mnt/company_docs \
  --output /var/digrag/company_kb \
  --with-embeddings

# 毎週更新（インクリメンタル）
digrag build \
  --input /mnt/company_docs \
  --output /var/digrag/company_kb \
  --with-embeddings \
  --incremental

# MCP サーバー起動（本番）
digrag serve --index-dir /var/digrag/company_kb

# Claude Desktop で統合（複数ユーザー共有可）
```

### ユースケース 3: 研究論文集の検索

**シナリオ**: 500 PDF をテキスト化したドキュメント、ハイブリッド検索

```bash
# Step 1: PDF をテキスト化（digrag は Markdown/Text 対応）
# 外部ツール（pandoc など）で PDF → Markdown 変換
for pdf in papers/*.pdf; do
  pandoc "$pdf" -o "papers_text/${pdf%.pdf}.md"
done

# Step 2: インデックス構築
digrag build \
  --input papers_text \
  --output ~/.digrag/papers \
  --with-embeddings

# Step 3: ハイブリッド検索で利用
digrag search "深層学習 言語モデル" \
  --index-dir ~/.digrag/papers \
  --mode hybrid \
  --top-k 10

# 論文を効率的に発見・参考文献の追跡
```

---

## トラブルシューティング

### Q: Embeddings API コストが予想より高い

**原因**: 毎回フル構築している

**解決策**:
```bash
# インクリメンタルビルド使用
digrag build --input ~/notes --with-embeddings --incremental
```

**効果**: 最大 99% コスト削減（変更分のみ処理）

---

### Q: セマンティック検索が動作しない

**原因 1**: `OPENROUTER_API_KEY` が設定されていない

```bash
# 確認
echo $OPENROUTER_API_KEY

# 設定
export OPENROUTER_API_KEY="sk-or-v1-..."
```

**原因 2**: インデックスに Vector Index がない

```bash
# 確認
ls -la ~/.digrag/index/
# faiss_index.json が無い場合は以下で再構築

digrag build --input ~/notes --output ~/.digrag/index --with-embeddings
```

---

### Q: MCP サーバーが起動しない（Claude Code）

**原因**: パス設定が誤っている

```bash
# digrag コマンドが PATH に存在するか確認
which digrag
# /usr/local/bin/digrag

# インデックスディレクトリが存在するか確認
ls -la ~/.digrag/index/

# .mcp.json で絶対パス指定
{
  "digrag": {
    "command": "/usr/local/bin/digrag",
    "args": ["serve", "--index-dir", "/Users/username/.digrag/index"]
  }
}
```

---

### Q: 検索結果が期待と異なる

| 状況 | 解決策 |
|---|---|
| キーワードが含まれているのに結果に無い | `--mode bm25` で確認、トークン化の問題かも |
| セマンティック検索で関連性が低い | クエリを詳しく書き換える、別の言い方を試す |
| ハイブリッドで一方の検索結果が優位 | RRF のパラメータ k を調整（デフォルト 60） |

---

### Q: 大規模インデックスが遅い

**最適化方法**:

1. **バッチサイズ調整**
   ```toml
   [embedding]
   batch_size = 100  # デフォルト 50、環境に応じて増減
   ```

2. **並列処理**
   - 複数 CPU コアを活用
   - 設定: `num_workers` パラメータ（オプション）

3. **キャッシュ活用**
   - Query cache: `/var/digrag/query_cache.db`
   - 同じクエリの再実行が高速化

---

## 参考資料

- **GitHub**: https://github.com/takets/digrag
- **OpenRouter API**: https://openrouter.ai
- **MCP Spec**: https://modelcontextprotocol.io
- **BM25 アルゴリズム**: https://en.wikipedia.org/wiki/Okapi_BM25
- **FAISS**: https://github.com/facebookresearch/faiss

---

## ライセンス

MIT License - 詳細は [LICENSE](LICENSE) を参照

---

**最終更新**: 2024-12-29
