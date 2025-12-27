# digrag: ポータブル テキスト RAG 検索エンジン

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[English](README.md)

あらゆるテキストファイルに対応した高性能でポータブルな RAG（Retrieval-Augmented Generation）検索エンジンです。メモ、ノート、ドキュメントなどのテキストコンテンツを BM25 キーワード検索、セマンティック（ベクトル）検索、またはハイブリッド検索で検索できます。MCP（Model Context Protocol）を通じて Claude とシームレスに統合できます。

## 特徴

- **日本語対応**: Lindera IPADIC による完全な日本語トークン化
- **MCP 統合**: Claude Code および Claude Desktop に検索機能を提供
- **高速検索**: BM25 キーワード検索（クエリあたり約 30 マイクロ秒）
- **セマンティック検索**: OpenRouter 埋め込み API によるベクトルベースの意味検索
- **ハイブリッド検索**: Reciprocal Rank Fusion で BM25 とセマンティック結果を統合
- **ポータブルバイナリ**: macOS、Linux、Windows 向け単一バイナリ（約 70MB）
- **クエリ書き換え**: キャッシュ付き LLM ベースのクエリ最適化

## インストール

### バイナリダウンロード

[GitHub Releases](https://github.com/takets/digrag/releases) から最新バイナリをダウンロード:

| プラットフォーム | ダウンロード |
|----------------|-------------|
| macOS (Apple Silicon) | `digrag-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `digrag-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `digrag-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `digrag-x86_64-pc-windows-msvc.zip` |

```bash
# 例: macOS Apple Silicon
curl -LO https://github.com/takets/digrag/releases/latest/download/digrag-aarch64-apple-darwin.tar.gz
tar xzf digrag-aarch64-apple-darwin.tar.gz
sudo mv digrag /usr/local/bin/
digrag --version
```

### install.sh を使用

```bash
curl -sSL https://raw.githubusercontent.com/takets/digrag/main/install.sh | bash
```

### cargo install

Rust 1.70 以上がインストールされている場合:

```bash
cargo install digrag
```

### ソースからビルド

```bash
git clone https://github.com/takets/digrag.git
cd digrag
make build-release
./target/release/digrag --version
```

## クイックスタート

### 1. 設定の初期化

```bash
digrag init
```

これにより、`~/.config/digrag/config.toml`（Linux/macOS）または `%APPDATA%\digrag\config.toml`（Windows）に設定ファイルが作成されます。

### 2. インデックス構築

```bash
# BM25 のみ（高速、API キー不要）
digrag build --input ~/notes --output ~/.digrag/index

# セマンティック埋め込み付き（OPENROUTER_API_KEY が必要）
export OPENROUTER_API_KEY="sk-or-v1-..."
digrag build --input ~/notes --output ~/.digrag/index --with-embeddings
```

### 3. 検索

```bash
# BM25 検索
digrag search "会議メモ" --index-dir ~/.digrag/index

# ハイブリッド検索
digrag search "プロジェクトのアイデア" --index-dir ~/.digrag/index --mode hybrid

# セマンティック検索
digrag search "類似のコンセプト" --index-dir ~/.digrag/index --mode semantic
```

## MCP セットアップ

### Claude Code

MCP 設定（`.mcp.json` または設定画面）に追加:

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

### Claude Desktop

`~/.claude/claude_desktop_config.json` に追加:

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

### 利用可能な MCP ツール

設定後、Claude は以下のツールを使用できます:

| ツール | 説明 |
|-------|------|
| `query_memos` | BM25、セマンティック、またはハイブリッドモードでドキュメントを検索 |
| `list_tags` | ドキュメント数とともに利用可能なすべてのタグを一覧表示 |
| `get_recent_memos` | 最近更新されたドキュメントを取得 |

### 初期セットアップ手順

```bash
# 1. 設定を初期化
digrag init

# 2. インデックスを構築
digrag build --input ~/Documents/notes --output ~/.digrag/index --with-embeddings

# 3. サーバーを手動でテスト（オプション）
digrag serve --index-dir ~/.digrag/index

# 4. Claude Code または Claude Desktop を設定（上記参照）
```

## コマンドリファレンス

### init

digrag 設定ファイルを初期化します。

```bash
digrag init [OPTIONS]
```

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|-------|------|-----------|
| `--force` | `-f` | 既存の設定を上書き | `false` |

### serve

MCP サーバーを起動します（stdin/stdout で通信）。

```bash
digrag serve [OPTIONS]
```

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|-------|------|-----------|
| `--index-dir` | `-i` | インデックスディレクトリのパス | `.rag` |

### build

テキストファイルから検索インデックスを構築します。

```bash
digrag build --input <PATH> [OPTIONS]
```

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|-------|------|-----------|
| `--input` | `-i` | ソースファイルまたはディレクトリ（必須） | - |
| `--output` | `-o` | 出力インデックスディレクトリ | `.rag` |
| `--with-embeddings` | - | 埋め込みを生成（`OPENROUTER_API_KEY` が必要） | `false` |
| `--skip-embeddings` | - | 埋め込み生成をスキップ（BM25 のみ） | `false` |

### search

コマンドラインからインデックスを検索します（テスト用）。

```bash
digrag search <QUERY> [OPTIONS]
```

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|-------|------|-----------|
| `<query>` | - | 検索クエリ（必須） | - |
| `--index-dir` | `-i` | インデックスディレクトリのパス | `.rag` |
| `--top-k` | `-k` | 返す結果の数 | `10` |
| `--mode` | `-m` | 検索モード: `bm25`, `semantic`, `hybrid` | `bm25` |
| `--tag` | `-t` | タグでフィルタ | - |

### グローバルオプション

| オプション | 短縮形 | 説明 |
|-----------|-------|------|
| `--verbose` | `-v` | 詳細ログを有効化 |
| `--help` | `-h` | ヘルプ情報を表示 |
| `--version` | `-V` | バージョンを表示 |

## ユースケース

- **個人メモ検索**: Markdown ノート、日記、メモを検索
- **ドキュメント検索**: プロジェクトドキュメント、Wiki、ナレッジベースをインデックス化して検索
- **学習資料**: 学習ノート、研究論文、参考資料を整理・検索
- **コードドキュメント**: コードコメント、README、技術文書を検索

## 開発

### テスト

```bash
cargo test                          # すべてのテスト
cargo test --test compatibility_test  # 統合テスト
cargo test --lib                    # ユニットテスト
```

### ベンチマーク

```bash
cargo bench
```

結果:
- BM25 検索: クエリあたり約 30 マイクロ秒
- セマンティック検索: 約 3 ナノ秒（ベクトル検索）
- ハイブリッド検索: クエリあたり約 34 マイクロ秒

### コード品質

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## プロジェクト構成

```
digrag/
├── src/
│   ├── lib.rs                 # ライブラリエクスポート
│   ├── main.rs                # CLI バイナリ
│   ├── config/                # 設定構造体
│   ├── loader/                # ドキュメント読み込みとパース
│   ├── tokenizer/             # 日本語トークン化
│   ├── index/                 # 検索インデックス
│   ├── search/                # 検索統合
│   ├── embedding/             # OpenRouter API クライアント
│   ├── rewriter/              # クエリ書き換え
│   └── mcp/                   # MCP サーバー実装
├── tests/
│   └── compatibility_test.rs  # Python 互換性テスト
├── benches/
│   └── search_bench.rs        # パフォーマンスベンチマーク
├── Cargo.toml                 # 依存関係
└── README.md                  # このファイル
```

## ライセンス

MIT License - 詳細は [LICENSE](LICENSE) ファイルを参照してください。
