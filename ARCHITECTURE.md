# digrag アーキテクチャ詳細

---

## 目次

1. [プロジェクト概要](#プロジェクト概要)
2. [アーキテクチャダイアグラム](#アーキテクチャダイアグラム)
3. [コンポーネント詳細](#コンポーネント詳細)
4. [データフロー](#データフロー)
5. [技術スタック](#技術スタック)

---

## プロジェクト概要

### digrag とは

**digrag** は、テキストベースのナレッジを高速に検索するためのポータブル RAG（Retrieval-Augmented Generation）エンジンです。

| 特性 | 説明 |
|---|---|
| **言語** | Rust 1.70+ |
| **実行形式** | スタンドアロンバイナリ（~70MB） |
| **対応 OS** | macOS (Apple Silicon / Intel)、Linux (x86_64)、Windows |
| **統合方式** | CLI コマンド + MCP サーバー |
| **言語サポート** | 日本語（Lindera IPADIC）+ 英語その他 |

### 主要機能

- **BM25 キーワード検索**: ~30 μs/クエリの高速検索
- **セマンティック検索**: OpenRouter Embeddings による意味ベース検索
- **ハイブリッド検索**: BM25 と Semantic の結果を RRF で統合
- **インクリメンタルビルド**: SHA256 ハッシュで変更検出、API コスト 99% 削減
- **MCP 統合**: Claude Code / Claude Desktop から直接利用可能
- **複数抽出戦略**: Snippet / ChangelogEntry / Full のコンテンツ抽出

---

## アーキテクチャダイアグラム

### 層状アーキテクチャ

```
┌────────────────────────────────────────────────────────────────┐
│                          ユーザーインターフェース層                │
├────────────────┬────────────────────────────────┬──────────────┤
│   CLI コマンド   │   (今後) Web UI              │   MCP Server │
│ (init/build/  │                            │ (Claude Code/ │
│  search/serve)│                            │  Desktop)     │
└────┬───────────┴────────────────┬───────────────┴──────┬───────┘
     │                            │                      │
┌────┴────────────────────────────┴──────────────────────┴───────┐
│                       サーバー層 / API 層                         │
├─────────────────────────────────────────────────────────────────┤
│  • DigragMcpServer      → MCP サーバー実装                      │
│  • HTTP REST API        → (今後実装予定)                       │
│  • gRPC Server          → (今後実装予定)                       │
└────┬────────────────────────────────────────────────────────────┘
     │
┌────┴────────────────────────────────────────────────────────────┐
│                    検索・ロジック層                              │
├─────────────────────────────────────────────────────────────────┤
│  • Searcher             → 検索統合インターフェース               │
│  • Bm25Searcher         → BM25 検索実装                         │
│  • VectorSearcher       → ベクトル検索実装                      │
│  • ReciprocalRankFusion → ハイブリッド結果融合 (RRF)           │
│  • QueryRewriter        → LLM ベースクエリ書き換え             │
│  • ContentExtractor     → 複数戦略によるコンテンツ抽出          │
│  • Summarizer           → ルール/LLM ベース要約                │
└────┬────────────────────────────────────────────────────────────┘
     │
┌────┴────────────────────────────────────────────────────────────┐
│                    インデックス層                                 │
├─────────────────────────────────────────────────────────────────┤
│  BM25Index              → TF-IDF + BM25 アルゴリズム            │
│  ├─ inversed_index      → Map<Token, List<DocId>>             │
│  ├─ idf_weights         → Map<Token, IDF_Score>               │
│  └─ doc_lengths         → Map<DocId, Length>                  │
│                                                                 │
│  VectorIndex (FAISS)    → ベクトル埋め込み管理                  │
│  ├─ vectors             → List<Vec<f32>>                       │
│  ├─ doc_mapping         → Map<VectorId, DocId>                │
│  └─ vector_dim          → 1024 (mxbai-embed-large)            │
│                                                                 │
│  Docstore               → ドキュメント本体 + メタデータ          │
│  ├─ documents           → Map<DocId, Document>                │
│  └─ metadata            → Map<DocId, Metadata>                │
│                                                                 │
│  IndexMetadata          → インクリメンタル追跡用               │
│  ├─ content_hashes      → Map<DocId, SHA256>                  │
│  ├─ build_timestamp     → DateTime<Utc>                       │
│  └─ version             → u32                                 │
└────┬────────────────────────────────────────────────────────────┘
     │
┌────┴────────────────────────────────────────────────────────────┐
│                   データ処理・外部連携層                          │
├─────────────────────────────────────────────────────────────────┤
│  • ChangelogLoader      → Markdown / JSONL ファイル読み込み     │
│  • Tokenizer            → 日本語トークン化 (Lindera IPADIC)    │
│  • Document Parser      → タイトル・タグ・テキスト抽出          │
│  • OpenRouterEmbedding  → Embeddings API (生成/推論)          │
│  • OpenRouterClient     → Chat Completions API (LLM)          │
│  • QueryCache           → SQLite キャッシュ (クエリ書き換え)   │
└────┬────────────────────────────────────────────────────────────┘
     │
┌────┴────────────────────────────────────────────────────────────┐
│                    永続化層                                      │
├─────────────────────────────────────────────────────────────────┤
│  ファイルシステム                                               │
│  └─ ~/.digrag/index/                                          │
│     ├─ bm25_index.json         (BM25 インデックス)             │
│     ├─ faiss_index.json        (Vector インデックス)           │
│     ├─ docstore.json           (ドキュメント & メタデータ)     │
│     ├─ metadata.json           (ビルド情報・増分追跡)          │
│     └─ hash_cache/             (コンテンツハッシュキャッシュ)   │
│                                                                 │
│  データベース                                                    │
│  └─ ~/.digrag/query_cache.db   (クエリ書き換えキャッシュ)      │
└────────────────────────────────────────────────────────────────┘
```

### コンポーネント相互関係図

```
┌─────────────────────────────────────────────────────────────┐
│                 メイン エントリーポイント                       │
│                    main.rs                                   │
└────┬─────────────────────────────────────────────────────────┘
     │
     ├─→ init command
     │   └─→ AppConfig::init() → config.toml を生成
     │
     ├─→ build command
     │   └─→ IndexBuilder
     │       ├─→ ChangelogLoader    (文書読み込み)
     │       ├─→ Tokenizer          (日本語処理)
     │       ├─→ Bm25Index          (BM25 インデックス)
     │       ├─→ OpenRouterEmbedding (Embeddings 生成)
     │       ├─→ VectorIndex        (FAISS ベクトル)
     │       ├─→ Docstore           (メタデータ保存)
     │       ├─→ IncrementalDiff     (変更検出)
     │       └─→ IndexMetadata      (メタデータ保存)
     │
     ├─→ search command
     │   └─→ Searcher
     │       ├─→ Bm25Index.search()
     │       ├─→ VectorIndex.search() + OpenRouterEmbedding
     │       ├─→ ReciprocalRankFusion.merge_results()
     │       ├─→ ContentExtractor.extract()
     │       └─→ Summarizer.summarize() (optional)
     │
     └─→ serve command
         └─→ DigragMcpServer
             ├─→ query_memos      (検索ツール)
             │   └─→ Searcher
             │       └─→ Summarizer (optional)
             ├─→ list_tags        (タグ一覧ツール)
             │   └─→ Docstore
             │
             └─→ get_recent_memos (最新文書取得ツール)
                 └─→ Docstore
```

---

## コンポーネント詳細

### 1. 読み込み層（Loader）

#### ChangelogLoader

| 責務 | ファイルパターン | 出力 |
|---|---|---|
| ファイル読み込み | `.md`, `.txt`, `.jsonl` | `Vec<Document>` |
| メタデータ抽出 | 例: `* Title YYYY-MM-DD` | `title, date, tags` |
| テキストパース | Markdown フロントマター対応 | `Document::text` |

**フロー**:
```
入力ファイル
  ↓
1. ファイルリスト取得 (walkdir)
2. ファイル読み込み (std::fs)
3. メタデータ抽出 (正規表現)
4. Document 構造体生成
  ↓
Vec<Document>
```

**例**:
```markdown
* Meeting Notes 2024-01-15
  Tags: meeting, january

  Topics discussed:
  - Project timeline
  - Budget allocation
```

解析結果:
```rust
Document {
    id: "abc123...",
    metadata: Metadata {
        title: "Meeting Notes",
        date: 2024-01-15,
        tags: vec!["meeting", "january"]
    },
    text: "Topics discussed:\n- Project timeline\n..."
}
```

### 2. トークン化層（Tokenizer）

#### 日本語トークン化（Lindera IPADIC）

| 処理 | 説明 |
|---|---|
| テキスト正規化 | 全角→半角、大文字→小文字変換 |
| 形態素解析 | 日本語を単語に分割（IPADIC 辞書） |
| ストップワード除去 | 助詞・助動詞など不要な単語削除 |
| 複合語処理 | 複数トークンの組み合わせ |

**例**:
```
入力: "機械学習は統計学の応用です"

処理:
1. 正規化: (小文字化など)
2. 形態素解析:
   - 機械 (noun)
   - 学習 (noun)
   - は (particle) ← ストップワード除去
   - 統計 (noun)
   - 学 (noun)
   - の (particle) ← ストップワード除去
   - 応用 (noun)
   - です (auxiliary) ← ストップワード除去

出力: ["機械", "学習", "統計", "学", "応用"]
```

**BM25 での使用**:
```
各トークンについて TF-IDF を計算し、
スコア = Σ IDF(token) × (k1 + 1) × tf / (tf + k1(1 - b + b * L/Lavg))
  ここで:
  - k1, b : BM25 パラメータ (k1=1.5, b=0.75)
  - tf : トークンの文書内出現頻度
  - L : 文書の長さ
  - Lavg : 全文書の平均長
```

### 3. インデックス層（Index）

#### BM25Index 構造

```rust
pub struct Bm25Index {
    inverted_index: HashMap<String, Vec<(DocId, Frequency)>>,
    idf_weights: HashMap<String, f32>,
    doc_lengths: HashMap<String, usize>,
    k1: f32,     // 1.5
    b: f32,      // 0.75
}
```

**検索フロー**:
```
クエリ "Python API"
  ↓
1. トークン化: ["python", "api"]
2. 各トークンについて:
   - inverted_index["python"] → [(doc_1, 3), (doc_5, 1), ...]
   - inverted_index["api"] → [(doc_2, 2), (doc_1, 1), ...]
3. 共通文書を統合:
   - doc_1: (python × IDF) × score1 + (api × IDF) × score2
   - doc_2: (api × IDF) × score2
   - doc_5: (python × IDF) × score1
4. スコア降順ソート
  ↓
結果: [(doc_1, 2.3), (doc_2, 1.2), (doc_5, 0.8)]
```

#### VectorIndex 構造（FAISS）

```rust
pub struct VectorIndex {
    vectors: Vec<Vec<f32>>,           // 埋め込みベクトル（1024 次元）
    doc_mapping: HashMap<usize, String>, // Vector ID → DocId
    vector_dim: usize,                 // 1024
}
```

**セマンティック検索フロー**:
```
クエリ "Python 機械学習"
  ↓
1. クエリをベクトル化:
   OpenRouterEmbedding.embed(query)
   → query_vector: [0.23, -0.15, ..., 0.45] (1024 次元)
2. FAISS で最近傍ベクトル検索:
   cosine_similarity(query_vector, vectors)
3. スコア降順で結果返却
   → [doc_123, doc_456, doc_789]
  ↓
結果: [(doc_123, 0.92), (doc_456, 0.87), (doc_789, 0.74)]
```

#### Docstore 構造

```rust
pub struct Docstore {
    documents: HashMap<String, Document>,
}

pub struct Document {
    id: String,
    metadata: Metadata,
    text: String,
}

pub struct Metadata {
    title: String,
    date: DateTime<Utc>,
    tags: Vec<String>,
}
```

**役割**: すべてのドキュメント本体とメタデータを保持

### 4. 検索層（Search）

#### ReciprocalRankFusion (RRF)

ハイブリッド検索時に BM25 と Semantic の結果を統合します。

**アルゴリズム**:
```
RRF_score(doc) = Σ 1 / (k + rank(doc))

デフォルト k=60

例:
BM25 結果:         Semantic 結果:
1. doc_A (rank 1)  1. doc_C (rank 1)
2. doc_B (rank 2)  2. doc_A (rank 2)
3. doc_C (rank 3)  3. doc_D (rank 3)

RRF スコア計算:
- doc_A: 1/(60+1) + 1/(60+2) = 0.0164 + 0.0161 = 0.0325
- doc_B: 1/(60+2) + 0 = 0.0161
- doc_C: 1/(60+3) + 1/(60+1) = 0.0159 + 0.0164 = 0.0323
- doc_D: 0 + 1/(60+3) = 0.0159

統合結果:
1. doc_A (0.0325)
2. doc_C (0.0323)
3. doc_B (0.0161)
4. doc_D (0.0159)
```

**利点**:
- ランク情報のみ使用（スコア絶対値に依存しない）
- BM25 と Semantic のスケール差を吸収
- 両検索で高ランクの文書が最終的に高ランク

### 5. インクリメンタルビルド層

#### IncrementalDiff

**処理フロー**:
```
新規ファイル群
  ↓
1. 各文書について SHA256(title + text) を計算
2. 前回の metadata.json から前回ハッシュを読み込み
3. 差分を検出:
   - 新規: 今回のみに存在
   - 更新: ハッシュが変化
   - 削除: 前回のみに存在
   - 未変: ハッシュが同じ
  ↓
DiffResult {
    added: Vec<Document>,
    modified: Vec<Document>,
    removed: Vec<String>,     // doc IDs
    unchanged: Vec<String>,
}
  ↓
4. 処理対象を決定:
   - BM25: added + modified を処理
   - Embeddings: added + modified のみ API 呼び出し
   - Docstore: すべてマージ (removed は削除)
```

**コスト削減効果**:
```
例: 640 文書, 毎週 7 文書変更

従来: 640 × embeddings_api_call = 640 回
      → 約 $0.80 (mxbai で @ $0.001/1k tokens)

インクリメンタル: 7 × embeddings_api_call = 7 回
                  → 約 $0.01

削減率: (640 - 7) / 640 = 98.9%
```

---

## データフロー

### ビルド時フロー

```
CLI: digrag build --input ~/notes --output ~/.digrag/index --with-embeddings --incremental

┌─────────────────────────────────────────────────────────────┐
│ Phase 1: ドキュメント読み込み                                │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         ChangelogLoader::load()
         ├─ walkdir で ~/notes 走査
         ├─ 各 .md ファイル読み込み
         ├─ メタデータ抽出（タイトル・タグ・日付）
         └─ Vec<Document> 返却

         ← ロード結果: 640 文書

┌─────────────────────────────────────────────────────────────┐
│ Phase 2: 差分検出（--incremental 時のみ）                    │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         IncrementalDiff::detect()
         ├─ 現在の doc → SHA256 ハッシュ
         ├─ 前回の metadata.json から前回ハッシュ読み込み
         ├─ 差分検出:
         │  ├─ Added: 5
         │  ├─ Modified: 2
         │  ├─ Deleted: 1
         │  └─ Unchanged: 632
         └─ 処理対象のみ抽出 (7 文書)

┌─────────────────────────────────────────────────────────────┐
│ Phase 3: BM25 インデックス構築                              │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         Tokenizer::tokenize()
         ├─ 日本語処理（Lindera IPADIC）
         └─ Vec<String> (tokens) 返却

                     ↓
         BM25Index::index()
         ├─ 逆索引構築: token → [doc_ids]
         ├─ IDF 計算: token → log(N / df)
         └─ 文書長記録: doc_id → length

┌─────────────────────────────────────────────────────────────┐
│ Phase 4: Vector インデックス構築（--with-embeddings）       │
└────────────────────┬────────────────────────────────────────┘
                     │
         [大量 API 呼び出し]
                     │
                     ↓
         OpenRouterEmbedding::embed() × 7
         ├─ 処理対象 7 文書のテキストをバッチで API 送信
         ├─ OpenRouter Embeddings API 呼び出し
         │  ├─ エンドポイント: https://openrouter.ai/api/v1/embeddings
         │  ├─ モデル: mixedbread-ai/mxbai-embed-large-v1
         │  └─ 出力: 1024 次元ベクトル
         └─ Vec<Vec<f32>> 返却

                     ↓
         VectorIndex::index()
         ├─ FAISS インデックス構築
         └─ doc → vector マッピング

┌─────────────────────────────────────────────────────────────┐
│ Phase 5: ファイル出力                                        │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         Serialize & Write
         ├─ bm25_index.json (BM25 インデックス)
         ├─ faiss_index.json (Vector インデックス)
         ├─ docstore.json (ドキュメント本体)
         └─ metadata.json (ビルド情報・次回増分用)

完了: ~/.digrag/index/ に全インデックスファイル出力
```

### 検索時フロー

```
CLI/MCP: digrag search "クエリ" --mode hybrid

┌─────────────────────────────────────────────────────────────┐
│ Phase 1: インデックスロード                                  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         Searcher::new()
         ├─ bm25_index.json ロード → BM25Index
         ├─ faiss_index.json ロード → VectorIndex
         ├─ docstore.json ロード → Docstore
         └─ Ready to search

┌─────────────────────────────────────────────────────────────┐
│ Phase 2: クエリ処理                                          │
└────────────────────┬────────────────────────────────────────┘
                     │
          ┌──────────┴──────────┐
          ↓                     ↓
     [BM25 検索]           [Semantic 検索]

┌─────────────────────────────────────────────────────────────┐
│ BM25 パス                                                   │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         1. Tokenizer::tokenize(query)
            "Python API" → ["python", "api"]

         2. BM25Index::search()
            inverted_index から該当文書を検索
            ├─ inverted_index["python"] → docs
            ├─ inverted_index["api"] → docs
            └─ マージしてスコア計算

         3. 結果: Vec<(DocId, BM25Score)>
            [(doc_1, 2.3), (doc_5, 1.2), ...]

┌─────────────────────────────────────────────────────────────┐
│ Semantic パス                                                │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         1. OpenRouterEmbedding::embed(query)
            API: OpenRouter Embeddings
            → query_vector: [0.23, -0.15, ..., 0.45]

         2. VectorIndex::search()
            FAISS で cosine_similarity 計算
            → 全 640 ベクトルとの類似度

         3. 結果: Vec<(DocId, SimilarityScore)>
            [(doc_2, 0.92), (doc_7, 0.87), ...]

         ↓

┌─────────────────────────────────────────────────────────────┐
│ Phase 3: ハイブリッド融合（--mode hybrid）                  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         ReciprocalRankFusion::merge()
         ├─ BM25 結果をランク化
         ├─ Semantic 結果をランク化
         └─ RRF スコア計算

         結果: Vec<(DocId, RRFScore)>
         [(doc_1, 0.035), (doc_2, 0.032), ...]

┌─────────────────────────────────────────────────────────────┐
│ Phase 4: コンテンツ抽出 & 要約                              │
└────────────────────┬────────────────────────────────────────┘
                     │
         各結果について:

         1. Docstore::get(doc_id)
            → Document 本体取得

         2. ContentExtractor::extract()
            策略別に処理:
            ├─ snippet: 最初 150 字
            ├─ entry: ChangelogEntry パターン抽出
            └─ full: 全文（max_chars 制限）

         3. Summarizer::summarize() (optional)
            ├─ ルールベース: 統計情報のみ
            └─ LLM ベース: OpenRouter Chat API で生成

┌─────────────────────────────────────────────────────────────┐
│ Phase 5: 結果返却                                            │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓
         QueryMemosResponse {
             results: Vec<MemoResult> [
                 {
                     id: "doc_1",
                     title: "...",
                     date: "2024-01-15",
                     tags: ["tag1", "tag2"],
                     snippet: "...",
                     score: 0.92
                 },
                 ...
             ],
             total: 3
         }

         CLI: 画面に出力
         MCP: Claude に返却
```

---

## 技術スタック

### 言語・フレームワーク

| レイヤー | 技術 | 用途 |
|---|---|---|
| **言語** | Rust 1.70+ | 高性能、メモリ安全 |
| **비동期ランタイム** | Tokio 1.42 | 非同期処理、I/O |
| **MCP SDK** | rmcp 0.1 | Claude 統合 |

### コア処理ライブラリ

| ライブラリ | バージョン | 用途 |
|---|---|---|
| **lindera** | 1.4 | 日本語トークン化（IPADIC） |
| **bm25** | 0.3 | BM25 スコアリング |
| **ndarray** | 0.16 | ベクトル計算 |
| **sha2** | 0.10 | SHA256 ハッシング |

### API クライアント・外部連携

| ライブラリ | バージョン | 用途 |
|---|---|---|
| **reqwest** | 0.12 | HTTP クライアント（OpenRouter API） |
| **serde** | 1.0 | JSON シリアライズ |
| **serde_json** | 1.0 | JSON パース |

### ファイル・設定管理

| ライブラリ | バージョン | 用途 |
|---|---|---|
| **walkdir** | 2 | ディレクトリ走査 |
| **directories** | 5.0 | OS 別設定ディレクトリ |
| **toml** | 0.8 | TOML 設定ファイル |
| **rusqlite** | 0.32 | SQLite キャッシュ |
| **tempfile** | 3.14 | テストファイル |

### インターフェース・CLI

| ライブラリ | バージョン | 用途 |
|---|---|---|
| **clap** | 4.5 | CLI パーサー（derive） |
| **tracing** | 0.1 | ログ・テレメトリ |
| **tracing-subscriber** | 0.3 | ロギングバックエンド |

### テスト・ベンチマーク

| ライブラリ | バージョン | 用途 |
|---|---|---|
| **criterion** | 0.5 | ベンチマークテスト |
| **insta** | 1.41 | スナップショットテスト |
| **wiremock** | 0.6 | HTTP モック |
| **tokio-test** | 0.4 | 非同期テスト |

### パフォーマンス特性

| 操作 | 処理時間 | 備考 |
|---|---|---|
| **BM25 検索** | ~30 μs | クエリあたり |
| **Semantic 検索** | ~3 ns | ベクトル検索のみ（クエリエンベッド済み） |
| **Hybrid 検索** | ~34 μs | BM25 + ベクトル（クエリエンベッド時） |
| **インデックス構築** | ~35 秒 | 640 文書、Embeddings 生成含む |
| **ファイル読み込み** | ~100 ms | 640 文書 |

### バイナリサイズ

| ターゲット | サイズ | 備考 |
|---|---|---|
| macOS (aarch64) | ~70 MB | リリースビルド、strip 済み |
| Linux (x86_64) | ~72 MB | リリースビルド、strip 済み |
| Windows (x86_64) | ~75 MB | .exe ファイル |

LTO（Link Time Optimization）と codegen-units=1 で最適化。

---

## パフォーマンス最適化の考え方

### 1. インデックス層の最適化

- **BM25**: HashMap ベースで O(1) + O(log n) スコア計算
- **Vector**: FAISS で大規模ベクトル検索を高速化
- **Docstore**: HashMap で O(1) ルックアップ

### 2. API コスト削減

- **インクリメンタルビルド**: 変更分のみ Embeddings API 呼び出し
- **バッチ処理**: 複数文書をまとめて API 送信
- **キャッシュ**: クエリ書き換え結果を SQLite に保存

### 3. メモリ効率

- **Rust**: 所有権制度で自動メモリ管理
- **ベクトル**: 32 bit float (f32) で 1024 次元 = 4KB/ベクトル

---

**最終更新**: 2024-12-29
