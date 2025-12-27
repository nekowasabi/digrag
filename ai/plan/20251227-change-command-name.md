---
mission_id: null
title: "digrag プロジェクト統一化とドキュメント更新"
status: completed
progress: 100
phase: completed
tdd_mode: false
blockers: 0
created_at: "2025-12-27"
updated_at: "2025-12-27"
---

# Commander's Intent

## Purpose
- プロジェクト名を「digrag」（汎用テキストRAG）に統一し、READMEを汎用的なドキュメント検索ツールとして説明を更新
- MCPの設定方法を明記し、Claude Code・Claude Desktop での利用方法を示す
- コマンドオプションの詳細説明を追加
- 日本語ユーザーへの対応として README_ja.md を新規作成

## End State
- README.md が digrag の用途・機能・使用方法・MCP設定を完全に説明している
- README_ja.md が日本語版として存在し、README.md と相互リンク
- CLIコマンド（init, serve, build, search）の各オプション説明が充実
- MCP設定が JSON 形式で環境変数含めて記載されている
- インストール方法（バイナリ、cargo install、ソースビルド）が明記

## Key Tasks
- README.md を汎用テキストRAGとして説明文言を統一・更新
- MCP設定セクションを追加（Claude Code・Claude Desktop向け）
- CLIコマンドリファレンス（init, serve, build, search）を追加
- インストール手順セクションを拡充
- README_ja.md を新規作成（日本語版）
- 相互リンクを設置

## Constraints
- テンプレートのフォーマットを厳密に守ること
- 新規ファイル作成は README_ja.md のみ
- 既存コード（src/）は変更しない
- ドキュメントのみの更新

## Restraints
- README.md の既存内容（ライセンス、リポジトリURL等）は保持
- コマンドの説明は main.rs の実装に基づく
- 日本語版は英語版と同じ構成で作成

---

# Context

## 概要
- digrag は、メモやノート、ドキュメントなど任意のテキストファイルに対する汎用的な RAG（Retrieval-Augmented Generation）ツール
- BM25 キーワード検索と OpenRouter 埋め込み API による セマンティック検索に対応
- MCP（Model Context Protocol）による Claude との統合で、AI アシスタント内で検索結果を利用可能
- Rust 実装により、単一バイナリで Windows/macOS/Linux に対応

## 必須のルール
- このドキュメントはプロジェクト計画書であり、テンプレートフォーマットを厳密に守ること
- 日本語での説明
- 作成すべき文書と実装の対応を明確にすること

## 開発のゴール
- 汎用的なテキスト RAG として、ユーザーが自分のメモやドキュメントを効率的に検索・分析できる環境を提供
- Claude（Code / Desktop）との統合により、AI アシスタントの能力を拡張
- インストール・設定の簡潔さにより、初心者でも導入可能

---

# References

| @ref | @target | @test |
|------|---------|-------|
| /home/takets/repos/private_dotfiles/ai_doc/template.md | /home/takets/repos/digrag/PLAN.md | - |
| /home/takets/repos/digrag/README.md | /home/takets/repos/digrag/README.md | - |
| /home/takets/repos/digrag/src/main.rs | README.md, README_ja.md | - |
| /home/takets/repos/digrag/Cargo.toml | README.md | - |

---

# Progress Map

| Process | Status | Progress | Phase | Notes |
|---------|--------|----------|-------|-------|
| Process 1: 調査・情報収集 | completed | ▓▓▓▓▓ 100% | Discovery | テンプレート形式の確認、CLI 実装の確認 |
| Process 2: README.md 更新計画 | completed | ▓▓▓▓▓ 100% | Implementation | 汎用テキスト RAG の説明に統一 |
| Process 3: MCP 設定セクション追加 | completed | ▓▓▓▓▓ 100% | Implementation | Claude Code / Desktop の設定方法 |
| Process 4: CLI コマンドリファレンス追加 | completed | ▓▓▓▓▓ 100% | Implementation | init, serve, build, search の詳細説明 |
| Process 5: インストール方法の拡充 | completed | ▓▓▓▓▓ 100% | Implementation | バイナリ・cargo install・ソースビルド |
| Process 6: README_ja.md 作成 | completed | ▓▓▓▓▓ 100% | Implementation | 日本語版ドキュメント |
| **Overall** | **completed** | **▓▓▓▓▓ 100%** | **completed** | **Blockers: 0** |

---

# Processes

## Process 1: 調査・情報収集

### Discovery Phase: 現状分析

#### 1.1 README.md の現状分析
- **現在の説明**: "cl-search: Rust-based Changelog Search Engine"
- **問題点**:
  - プロジェクト名が古い（「cl-search」）
  - Cargo.toml では既に「digrag」に統一済み
  - ドキュメント内容が changelog / memo 中心で、汎用テキスト RAG として説明されていない
- **対応方針**:
  - タイトルを「digrag: Portable Text RAG Search Engine」に統一
  - 説明文を汎用的な文書検索・分析ツールとして書き換え
  - メモ・ノート・ドキュメント等、任意のテキストファイルが対象であることを明記

#### 1.2 Cargo.toml の確認
- **name**: "digrag" ✓ 統一済み
- **description**: "Portable RAG search engine with BM25 and semantic search for any directory" ✓
- **keywords**: mcp, search, bm25, semantic, rag ✓
- **version**: 0.1.0

#### 1.3 CLI 実装の確認（main.rs より）
確認されたコマンド：
1. **init** - 設定ファイルの初期化
   - `--force` オプションで既存ファイルを上書き
   - デフォルト位置: ユーザーの設定ディレクトリ

2. **serve** - MCP サーバーの起動
   - `--index-dir` : インデックスディレクトリ指定（デフォルト: .rag）
   - stdin/stdout を使用した MCP stdio トランスポート

3. **build** - インデックス構築
   - `--input` : 対象ファイル/ディレクトリ（必須）
   - `--output` : インデックス出力先（デフォルト: .rag）
   - `--with-embeddings` : OpenRouter API で埋め込み生成
   - `--skip-embeddings` : BM25 のみ

4. **search** - コマンドライン検索（テスト用）
   - 引数: 検索クエリ
   - `--index-dir` : インデックスディレクトリ（デフォルト: .rag）
   - `--top-k` : 返す結果数（デフォルト: 10）
   - `--mode` : 検索モード（bm25/semantic/hybrid, デフォルト: bm25）
   - `--tag` : タグでフィルタ

#### 1.4 MCP サーバー機能（main.rs より）
実装されているツール：
1. **query_memos** - 検索実行
   - query: 検索文字列
   - top_k: 結果数（デフォルト: 10）
   - mode: 検索モード（デフォルト: bm25）
   - tag_filter: タグフィルタ

2. **list_tags** - タグ一覧表示

3. **get_recent_memos** - 最新メモ取得
   - limit: 件数（デフォルト: 10）

✅ **Discovery Complete**

---

## Process 2: README.md 更新計画

### Design Phase: 構成設計

#### 2.1 更新する主要セクション

**セクション 1: タイトル・説明**
- 現在: "cl-search: Rust-based Changelog Search Engine"
- 更新: "digrag: Portable Text RAG Search Engine"
- 説明文を汎用テキスト RAG として説明

**セクション 2: Features（機能）**
- 既存内容を保持しつつ、汎用性を強調
- Japanese Support を先頭に配置
- MCP Integration を重視

**セクション 3: Quick Start**
- ビルド方法は現在通り
- インデックス構築例を汎用的に変更（changelogmemo ではなく、任意のテキストファイル）
- 検索例を複数パターン示す

**セクション 4: Installation（新規追加）**
- Binary Download: GitHub Releases からのダウンロード方法
- cargo install: Crates.io からのインストール
- Build from Source: ソースからのビルド

**セクション 5: MCP Setup（新規追加）**
- Claude Code での設定方法（JSON 形式）
- Claude Desktop での設定方法（JSON 形式）
- 環境変数（OPENROUTER_API_KEY）の説明
- 各設定での動作説明

**セクション 6: Command Reference（新規追加）**
- init コマンド
- serve コマンド
- build コマンド
- search コマンド
  各コマンドのオプション詳細を表形式で整理

**セクション 7: Use Cases（新規追加）**
- 個人メモの検索
- 社内ドキュメント検索
- 学習資料の管理
- プロジェクトドキュメント検索

✅ **Design Complete**

---

## Process 3: MCP 設定セクション追加

### Design Phase: MCP 統合方法の設計

#### 3.1 MCP セクション構成

**部分 3.1: Claude Code 設定**
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

**部分 3.2: Claude Desktop 設定**
- 設定ファイル位置: `~/.claude/claude_desktop_config.json`
- 同様の JSON フォーマット

**部分 3.3: ツール説明**
- query_memos: 検索実行（BM25/Semantic/Hybrid）
- list_tags: タグ一覧
- get_recent_memos: 最新メモ取得

**部分 3.4: 初期設定手順**
```bash
digrag init           # 設定ファイル作成
digrag build --input ./docs --output ~/.digrag/index  # インデックス構築
digrag serve --index-dir ~/.digrag/index  # サーバー起動（手動確認用）
```

✅ **Design Complete**

---

## Process 4: CLI コマンドリファレンス追加

### Design Phase: コマンドドキュメント設計

#### 4.1 コマンド一覧と説明

**init コマンド**
| オプション | 説明 | デフォルト |
|-----------|------|----------|
| --force, -f | 既存設定ファイルを上書き | false |

機能: ユーザー設定ディレクトリに config.toml を作成

**serve コマンド**
| オプション | 説明 | デフォルト |
|-----------|------|----------|
| --index-dir, -i | インデックスディレクトリ | .rag |

機能: MCP サーバーを起動し、stdin/stdout でクライアントと通信

**build コマンド**
| オプション | 説明 | デフォルト |
|-----------|------|----------|
| --input, -i | 対象ファイル/ディレクトリ（必須） | - |
| --output, -o | インデックス出力先 | .rag |
| --with-embeddings | OpenRouter API で埋め込み生成 | false |
| --skip-embeddings | BM25 のみ構築 | false |

機能: テキストファイルからインデックスを構築（埋め込みオプション）

**search コマンド**
| オプション | 説明 | デフォルト |
|-----------|------|----------|
| <query> | 検索クエリ（必須） | - |
| --index-dir, -i | インデックスディレクトリ | .rag |
| --top-k, -k | 返す結果数 | 10 |
| --mode, -m | 検索モード（bm25/semantic/hybrid） | bm25 |
| --tag, -t | タグでフィルタ | (なし) |

機能: コマンドラインから直接検索実行（テスト用）

✅ **Design Complete**

---

## Process 5: インストール方法の拡充

### Design Phase: インストール手段の整理

#### 5.1 インストール方法

**方法 1: バイナリダウンロード**
- GitHub Releases から最新バイナリをダウンロード
- 環境: macOS (Intel/Apple Silicon), Linux (x86_64), Windows (x86_64)
- 手順:
  1. Releases ページから対応するバイナリを選択
  2. 解凍して PATH に配置（例: `/usr/local/bin`）
  3. `digrag --version` で確認

**方法 2: cargo install**
- Crates.io から最新版をインストール
- 前提条件: Rust 1.70+
- 手順:
  ```bash
  cargo install digrag
  ```

**方法 3: ソースからビルド**
- リポジトリをクローンしてビルド
- 前提条件: Rust 1.70+, Git
- 手順:
  ```bash
  git clone https://github.com/takets/digrag.git
  cd digrag
  make build-release
  ./target/release/digrag --version
  ```

✅ **Design Complete**

---

## Process 6: README_ja.md 作成

### Design Phase: 日本語版ドキュメント構成

#### 6.1 README_ja.md 構成

- README.md と同一の構成
- すべての説明・例を日本語化
- コマンド例は両言語で表示可能
- 相互リンク:
  - README.md: 最下部に「[日本語版](README_ja.md)」
  - README_ja.md: 最下部に「[English](README.md)」

#### 6.2 日本語化のポイント

1. タイトル・見出しを日本語化
2. 説明文を自然な日本語に
3. 用語の統一:
   - changelog memo → 変更ログメモ
   - semantic search → セマンティック検索（またはベクトル検索）
   - hybrid search → ハイブリッド検索
4. 日本語での使用例を追加

✅ **Design Complete**

---

# Implementation Order

1. **Process 2**: README.md 全体の見直し・更新（タイトル、説明、構成変更）
2. **Process 3**: MCP セクションを README.md に追加
3. **Process 4**: CLI コマンドリファレンスセクションを README.md に追加
4. **Process 5**: インストール方法セクションを README.md に追加（Quick Start の前に移動）
5. **Process 6**: README_ja.md を新規作成（README.md の日本語版）

---

# Management

## Blockers

| ID | Description | Status | Resolution |
|----|-------------|--------|-----------|
| - | なし | - | - |

## Lessons

| ID | Insight | Severity | Applied |
|----|---------|----------|---------|
| - | 記録予定 | - | - |

## Feedback Log

| Date | Type | Content | Status |
|------|------|---------|--------|
| - | - | - | - |

## Completion Checklist

- [x] README.md が汎用テキスト RAG として説明更新完了
- [x] MCP セットアップセクション（Claude Code / Desktop）を追加
- [x] CLI コマンドリファレンス完成
- [x] インストール方法（3 種類）を整理
- [x] README_ja.md （日本語版）を新規作成
- [x] 相互リンク確認
- [x] すべてのセクションレビュー完了

---

<!--
Process 番号規則
- 1-9: 調査・設計
- 10-49: 実装・文書作成
- 50-99: テスト・レビュー
- 100+: 最終確認・マージ
-->
