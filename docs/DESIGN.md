---
title: 無変換スイッチ マルチプラットフォーム化 設計書（v3 — Rust統合版）
date: 2025-02-15
updated: 2025-02-15
tags:
  - keyboard-remapping
  - cross-platform
  - kanata
  - rust
summary: muhenkan-switchをkanata＋Rust製companionバイナリ構成でWindows/macOS/Linuxに対応させる設計書。JIS配列前提、macOS未検証。
---

## 概要

現行の `muhenkan-switch`（AutoHotkey v2、Windows専用）をマルチプラットフォーム化する。

**アーキテクチャ:** kanata（既存OSSキーリマッパー）＋ Rust製 companion バイナリ

**前提条件:**
- 対象OS: Windows / macOS / Linux
- macOS は設定ファイルを提供するが、**開発者の検証環境がないため未検証**
- **JIS配列キーボード前提**。US配列は考慮しない
- ライセンス: GPL-2.0（現行を継承）

**設計方針:**
- kanata を外部バイナリとして利用（クレート組み込みはしない）
- kanata と companion は `cmd` アクション（プロセス起動）で疎結合に接続
- companion は非同期なし・unsafe なし・ライフタイム注釈なしのシンプルな Rust コード

---

## 現行機能の分類と実装方針

### Layer 1: キー入力のインターセプト → kanata に委譲

| 機能 | kanata での実現 |
|------|----------------|
| 無変換キーの tap/hold 判定 | `tap-hold` アクション |
| 無変換+X → 別キー出力 | レイヤー定義 |
| HJKL → カーソル移動 | レイヤー内で `left` `down` `up` `right` |
| YUIO → 単語移動/行頭行末 | レイヤー内マクロ |
| NM → BackSpace/Delete | レイヤー内で `bspc` `del` |
| カンマ・ピリオド → 句読点 | unicode 出力 |

### Layer 2: OS連携 → companion バイナリ（Rust）

| 機能 | 実装方針 |
|------|----------|
| アプリ切り替え | OS別: PowerShell / wmctrl / osascript(未検証) |
| フォルダオープン | `open` クレート |
| 選択文字列 → Web検索 | `arboard`（クリップボード） + `webbrowser`（ブラウザ起動） |
| タイムスタンプ | `chrono` → `arboard`（クリップボード書き込み） |
| スクリーンショット | OS別コマンド呼び出し |

### Layer 3: 設定管理 → companion が config.toml を読み込み

- `toml` + `serde` で設定ファイルを構造体にデシリアライズ
- 検索URL、アプリ名、フォルダパス、タイムスタンプ形式を設定可能

---

## アーキテクチャ図

```
┌──────────────────────────────────────────┐
│            muhenkan-switch-rs            │
│                                          │
│  ┌──────────┐      ┌──────────────────┐ │
│  │  kanata  │─cmd─→│    companion     │ │
│  │  (.kbd)  │      │  (Rust binary)   │ │
│  └──────────┘      └──────────────────┘ │
│   Layer 1            Layer 2 + 3        │
│   キー入力           OS連携 + 設定管理  │
│                                          │
│  ┌──────────┐                            │
│  │ config   │← companion が読み込み     │
│  │ (.toml)  │                            │
│  └──────────┘                            │
└──────────────────────────────────────────┘
```

---

## 無変換キーのOS間対応

| OS | kanata キー名 | 備考 |
|----|--------------|------|
| Windows | `muhenkan` | VK 0x1D。JISキーボードで正常認識 |
| Linux | `muhenkan` | evdev `KEY_MUHENKAN` (keycode 102) |
| macOS | `eisu` (推定) | JIS配列Macの「英数」キー。**未検証** |

---

## companion CLI 仕様

```
companion <COMMAND> [OPTIONS]

Commands:
  search       --engine <NAME>    選択テキスト（クリップボード）をWeb検索
  switch-app   --target <NAME>    指定アプリを最前面に
  open-folder  --target <NAME>    指定フォルダを開く
  timestamp    --action <ACTION>  タイムスタンプ操作 (paste|copy|cut)
  screenshot                      ウィンドウキャプチャ
```

設定は実行ファイルと同じディレクトリの `config.toml` から読み込む。

---

## 使用クレート

| 用途 | クレート | バージョン |
|------|---------|-----------|
| CLI引数パース | `clap` (derive) | 4.x |
| クリップボード | `arboard` | 3.x |
| ブラウザ起動 | `webbrowser` | 1.x |
| ファイル/フォルダオープン | `open` | 5.x |
| TOML読み込み | `toml` + `serde` | 0.8.x / 1.x |
| 日時処理 | `chrono` | 0.4.x |
| URLエンコード | `urlencoding` | 2.x |
| エラーハンドリング | `anyhow` | 1.x |

---

## ビルドとリリース

- GitHub Actions で Windows (x64) / Linux (x64) / macOS (x64, aarch64) のバイナリを自動ビルド
- タグ push (`v*`) でリリース作成
- リリース zip には companion バイナリ + .kbd + config.toml を同梱
- kanata 本体は同梱またはダウンロードリンクを案内

---

## 実装ロードマップ

### Phase 1: kanata コアキーマッピング（1-2週間）
- `muhenkan.kbd` で無変換 + HJKL カーソル移動を実装
- Windows / Linux で動作確認

### Phase 2: companion 最小実装（2-3週間）
- 実装順序: open-folder → search → timestamp → switch-app
- kanata の `cmd` アクションとの結合テスト

### Phase 3: ビルド自動化 + リリース（1-2週間）
- GitHub Actions クロスコンパイル
- README + macOS 用 .kbd 作成

### Phase 4: 機能拡充（継続）
- ホットストリング、スクリーンショット、GUI（Tauri検討）
