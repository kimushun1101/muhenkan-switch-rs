# CLAUDE.md — Claude Code プロジェクト設定

## プロジェクト構成

Cargo ワークスペース（3 クレート）:

- `muhenkan-switch` — GUI（Tauri v2）。設定画面と kanata プロセス管理
- `muhenkan-switch-core` — CLI ツール。検索・アプリ切替・フォルダ・タイムスタンプ等の実行
- `muhenkan-switch-config` — 共有ライブラリ。TOML 設定の読み書き・バリデーション

フロントエンド (`muhenkan-switch/frontend/`) は **Vanilla JS**（HTML + CSS + JS のみ）。
Node.js やビルドステップは不要。

## マルチプラットフォーム対応

このプロジェクトは **Windows / Linux / macOS** の 3 環境で動作する。
コードやスクリプトを書く際は以下を守ること。

### シェルスクリプト (*.sh)

- **`sed -i` は直接使わない** — macOS の BSD sed は `sed -i ''` が必要。
  `sedi()` ヘルパー（`scripts/sync-kanata-version.sh` 参照）のようなラッパーを使うか、
  一時ファイル + mv のパターンで回避する。
- **改行コードは LF** — `.gitattributes` で `*.sh text eol=lf` を設定済み。
  新しいシェルスクリプトを追加する場合も同様に LF であることを確認する。
- **`find`, `grep`, `readlink` 等の GNU 拡張オプションに依存しない** —
  macOS のデフォルトは BSD 版。POSIX 互換のオプションを使う。
  例: `readlink -f` → macOS では動かない。`cd "$(dirname "$0")/.." && pwd` を使う。

### Rust / Cargo

- **OS 固有の依存は `[target.'cfg(...)'.dependencies]` で分離** —
  `windows` クレート等はターゲット条件付きで記述する。
- **パスセパレータ** — Rust コード内でパスを扱う場合は `std::path::Path` / `PathBuf` を使い、
  `/` や `\` をハードコードしない。
- **プラットフォーム分岐は `mod imp` パターン** — 同一ファイル内に
  `#[cfg(target_os = "windows")] mod imp { ... }` / `#[cfg(target_os = "linux")] mod imp { ... }`
  のようにまとめる（`muhenkan-switch-core/src/commands/` 参照）。
- **共有依存はワークスペース経由** — `anyhow`, `serde`, `chrono`, `open`,
  `muhenkan-switch-config` は `[workspace.dependencies]` で定義済み。
  各クレートでは `anyhow.workspace = true` のように参照する。

## コーディング規約

### エラーハンドリング

- `anyhow::Result<T>` を使う。独自エラー型は作らない。
- `.context()` / `.with_context()` でエラーメッセージに文脈を追加する。
- 致命的エラーは `anyhow::bail!()`。
- オプショナルな外部ツール（wmctrl, xdotool, notify-send 等）の失敗は
  `eprintln!()` で警告して続行する（graceful degradation）。

### テスト

- ファイル末尾に `#[cfg(test)] mod tests { ... }` ブロックを置く。
- 命名: `test_{対象}_{条件}` または `{関数名}_{シナリオ}`。
- 実行: `cargo test --workspace`。

### 言語

- **UI テキスト・ユーザー向けメッセージ**: 日本語
- **コード内コメント**: 英語可（セクション区切りは `// ── Section ──` 形式）
- **コミットメッセージ**: 日本語（技術用語は英語可）

## バージョン管理

- **Rust クレート**: `Cargo.toml` の `[workspace.package] version` が単一ソース。
- **kanata**: `kanata-version.txt` が単一ソース。
  `scripts/sync-kanata-version.sh` で 4 ファイルに同期する。
- バージョン等の定数を複数ファイルにハードコードしない。

## 開発コマンド

```bash
mise run build              # debug ビルド → bin/
mise run test               # cargo test --workspace
mise run dev                # ビルド + kanata 取得 + GUI 起動
mise run sync-kanata-version  # kanata バージョンを全ファイルに同期
mise run fetch-kanata       # 開発用 kanata バイナリをダウンロード
```
