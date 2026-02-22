---
title: デフォルト設定の最適化 調査結果
date: 2026-02-22
issue: "#24"
tags:
  - default-config
  - key-assignment
  - cross-platform
---

## 概要

`config/default.toml` のデフォルト設定（アプリ・検索・フォルダ・キー割り当て）を、より一般的で各 OS に適したものに見直すための調査。

---

## 1. 現在のデフォルトの問題点

### アプリセクション

| エントリ | key | process | 問題点 |
|---------|-----|---------|--------|
| editor | a | Code | OK（VS Code は広く普及） |
| word | w | WINWORD | 有償。未インストール環境で動作しない |
| email | e | OUTLOOK | 有償。未インストール環境で動作しない |
| slides | s | POWERPNT | 有償。未インストール環境で動作しない |
| pdf | d | SumatraPDF | ニッチ。多くのユーザーは未インストール |
| browser | f | firefox | OK |

- **有償ソフト前提**: Microsoft Office は有償で全ユーザーが持っているわけではない
- **Windows 専用**: macOS / Linux ユーザーにとっては全く使えない
- **カテゴリの偏り**: Office 系が3つ。ターミナルやコミュニケーションツールがない

### 検索セクション

| エントリ | key | 問題点 |
|---------|-----|--------|
| google | g | OK |
| ejje | q | OK |
| thesaurus | r | OK |
| translate | t | `t` はアプリのターミナルに使いたい（キー競合） |

### フォルダセクション

| エントリ | key | path | 問題点 |
|---------|-----|------|--------|
| documents | 1 | ~/Documents | OK |
| downloads | 2 | ~/Downloads | OK だが、最もアクセス頻度が高いのに2番 |
| desktop | 3 | ~/Desktop | OK |
| onedrive | 4 | ~/OneDrive | Windows 専用。macOS は iCloud、Linux は該当なし |
| trash | 5 | (空) | OK（OS 依存の特殊処理） |

---

## 2. OS 別の人気アプリ調査

### ブラウザ

| OS | 1位 | 2位 | 備考 |
|----|-----|-----|------|
| Windows | Chrome (`chrome`) | Edge (`msedge`) | Edge はプリインストール |
| macOS | Chrome (`Google Chrome`) | Safari (`Safari`) | Safari はプリインストール |
| Linux | Firefox (`firefox`) | Chrome (`google-chrome`) | Firefox は多くのディストリビューションにプリインストール |

### エディタ / IDE

| アプリ | Windows process | macOS process | Linux process | 普及度 |
|--------|----------------|---------------|---------------|--------|
| VS Code | `Code` | `Visual Studio Code` | `code` | 非常に高い（無償） |
| Cursor | `Cursor` | `Cursor` | `cursor` | 高い（AI エディタとして急成長） |
| JetBrains IDEs | `idea64` 等 | `IntelliJ IDEA` 等 | `jetbrains-idea` 等 | 高い（有償だが開発者に人気） |
| Sublime Text | `sublime_text` | `Sublime Text` | `sublime_text` | 中 |

### ターミナル

| OS | 推奨デフォルト | process | command | 理由 |
|----|--------------|---------|---------|------|
| Windows | Windows Terminal | `WindowsTerminal` | `wt` | Win11 プリインストール |
| macOS | Terminal | `Terminal` | `open -a Terminal` | プリインストール（iTerm2 は要インストール） |
| Linux | GNOME Terminal | `gnome-terminal` | `gnome-terminal` | Ubuntu/Fedora 等でプリインストール |

### コミュニケーション

| アプリ | Windows | macOS | Linux | 普及度 |
|--------|---------|-------|-------|--------|
| Slack | `slack` | `Slack` | `Slack` | 非常に高い（ビジネス） |
| Discord | `Discord` | `Discord` | `discord` | 高い（コミュニティ） |
| Teams | `ms-teams` | `Microsoft Teams` | 限定的 | 高い（企業） |

### ファイルマネージャ

| OS | アプリ | process | command |
|----|--------|---------|---------|
| Windows | Explorer | `explorer` | `explorer` |
| macOS | Finder | `Finder` | `open -a Finder` |
| Linux | Nautilus | `org.gnome.Nautilus` | `nautilus` |

### メモ / ナレッジ管理

| アプリ | Windows | macOS | Linux | 普及度 |
|--------|---------|-------|-------|--------|
| Obsidian | `Obsidian` | `Obsidian` | `obsidian` | 高い（無償、ローカルファースト） |
| Notion | `Notion` | `Notion` | ブラウザのみ | 高い（Web ベース） |

---

## 3. 検索エンジンの追加候補調査

### DeepL

- URL: `https://www.deepl.com/translator#auto/ja/{query}`
- 日本語翻訳の精度で Google 翻訳を上回ると評価されており、日本での利用者が非常に多い
- `{query}` パラメータで直接テキストを渡せる

### ChatGPT

- URL: `https://chatgpt.com/?q={query}&temporary-chat=true`
- `?q=` パラメータでプロンプトを事前入力可能（[コミュニティで確認](https://community.openai.com/t/query-parameters-in-chatgpt/1027747)）
- `&temporary-chat=true` で履歴を汚さない一時チャットを作成可能
- 選択テキストをそのまま AI に質問する使い方との相性が抜群

### Wikipedia

- URL: `https://ja.wikipedia.org/wiki/{query}`
- 用語の即時確認に有用
- 全 OS・全ブラウザで動作

### GitHub コード検索

- URL: `https://github.com/search?q={query}&type=code`
- 開発者向け。エラーメッセージや関数名をそのまま検索できる

---

## 4. kbd ファイルのキー制約

`muhenkan.kbd` で固定割り当てされているキーはディスパッチに使用できない。

### 固定キー（変更不可）

| キー | 用途 | 領域 |
|------|------|------|
| v | タイムスタンプ paste | 左手下段 |
| c | タイムスタンプ copy | 左手下段 |
| x | タイムスタンプ cut | 左手下段 |
| h, j, k, l | Vim カーソル移動 | 右手ホーム段 |
| u, i, y, o | 単語移動・行頭行末 | 右手上段 |
| n, m | BackSpace, Delete | 右手下段 |
| ; | Escape | 右手ホーム段 |
| , . | 句読点（、。） | 右手下段 |
| p | スクリーンショット | 右手上段 |
| f1 | 設定（将来） | ファンクション |

### ディスパッチ可能キー

現行: `1, 2, 3, 4, 5, q, r, t, g, a, w, e, s, d, f` (15個)
追加可能: `b` (kbd 変更必要), `z` (kbd 変更必要)

### 提案で発覚した競合

| 当初の提案 | 用途 | 実際の用途 | 結果 |
|-----------|------|-----------|------|
| `h` → DeepL | 検索 | 左カーソル移動（固定） | **競合 → `w` に変更** |
| `c` → ChatGPT | 検索 | タイムスタンプ copy（固定） | **競合 → `b` に変更（kbd 追加必要）** |

---

## 5. フォルダの利用頻度調査

一般的にファイルマネージャで頻繁に開かれるフォルダ:

| フォルダ | 頻度 | 理由 |
|---------|------|------|
| ~/Downloads | 非常に高 | ファイル DL 後に確認・移動。全ユーザー共通 |
| ~/Desktop | 高 | 一時的な作業ファイル置き場として多用 |
| ~/Documents | 中 | 書類の保存先。ただし実際にはプロジェクト別フォルダの方が多い |
| ~（ホーム） | 中 | ナビゲーションの起点 |
| Trash | 低〜中 | 誤削除の復元、ストレージ確認 |
| ~/OneDrive, iCloud | 人による | クラウドストレージを使わない人には不要。OS 依存 |

**結論:** フォルダは個人の作業スタイルに強く依存するため、普遍的なもの（Downloads, Desktop, Documents）+ カスタマイズ用のプレースホルダ（プロジェクトフォルダ）を提供する方針が適切。

---

## 6. 提案: 改定後のキー割り当て全体像

### 検索

| キー | エントリ | URL | ニーモニック |
|------|---------|-----|-------------|
| g | Google | `https://www.google.com/search?q={query}` | **G**oogle |
| q | 英辞郎 | `https://ejje.weblio.jp/content/{query}` | 辞書を**Q**uery |
| r | 類語辞典 | `https://thesaurus.weblio.jp/content/{query}` | **R**uigo（類語） |
| w | DeepL 翻訳 | `https://www.deepl.com/translator#auto/ja/{query}` | **W**eb翻訳 |
| b | ChatGPT | `https://chatgpt.com/?q={query}&temporary-chat=true` | AI **B**ot（kbd 追加必要） |

### フォルダ

| キー | エントリ | path | 備考 |
|------|---------|------|------|
| 1 | downloads | ~/Downloads | 最頻アクセス |
| 2 | desktop | ~/Desktop | 一時作業場 |
| 3 | documents | ~/Documents | 書類保存先 |
| 4 | project | ~/repos | **カスタマイズ推奨**。コメントで案内 |
| 5 | trash | (空) | OS 依存の特殊処理 |

### アプリ（OS 別）

| キー | 用途 | Windows | macOS | Linux | ニーモニック |
|------|------|---------|-------|-------|-------------|
| f | ブラウザ | Edge (`msedge`) | Safari | Firefox | **F**irefox / 閲覧 |
| a | エディタ | VS Code (`Code`) | VS Code (`Visual Studio Code`) | VS Code (`code`) | **A**pp |
| t | ターミナル | Windows Terminal (`WindowsTerminal`) | Terminal | GNOME Terminal (`gnome-terminal`) | **T**erminal |
| s | チャット | Slack (`slack`) | Slack (`Slack`) | Slack (`Slack`) | **S**lack |
| e | ファイラー | Explorer (`explorer`) | Finder (`Finder`) | Nautilus (`org.gnome.Nautilus`) | **E**xplorer |
| d | メモ | Obsidian (`Obsidian`) | Obsidian (`Obsidian`) | Obsidian (`obsidian`) | **D**ocument |

### 空きキー

| キー | 状態 | 備考 |
|------|------|------|
| z | 未使用（kbd 追加で利用可能） | ユーザーカスタマイズ用 |

---

## 7. 設計方針

### アプリ選定の基準

- **無償・プリインストール優先**: 有償ソフトはデフォルトに含めない
- **クロスプラットフォーム考慮**: OS 別にプリインストールされているアプリを優先
- **カテゴリバランス**: Office 偏重を解消し、ブラウザ・エディタ・ターミナル・チャット・ファイラー・メモの6カテゴリ

### 検索選定の基準

- **既存の仕組みで即対応可能**: URL に `{query}` を埋めるだけ
- **AI 検索の追加**: ChatGPT の `?q=` パラメータ対応を確認済み
- **翻訳は DeepL に変更**: 日本語翻訳の品質で支持が高い

### フォルダ選定の基準

- **普遍的なフォルダ + カスタマイズ枠**: 個人差が大きい領域なので、3つの標準フォルダ + 「自分用に変更してください」の枠
- **OS 依存の排除**: OneDrive / iCloud のような特定クラウドサービスをデフォルトにしない
- **頻度順**: Downloads（最頻）を1番に配置

### 実装方針

1. **kbd ファイルに `b` キーを追加**: `defsrc` と `mh-layer` に `dsp-b` を追加
2. **OS 別デフォルト設定ファイルの分離**: ビルド時または初回起動時に OS 判定して適切なデフォルトを生成
3. **単一 `default.toml` + コメント方式の廃止**: コメントでの代替案提示は見落とされやすい
4. **GUI スキーマの更新**: `desktop-schema.json`, `windows-schema.json` も連動して更新

---

## 参考リンク

- [ChatGPT URL query parameters](https://community.openai.com/t/query-parameters-in-chatgpt/1027747) - `?q=` パラメータの動作確認
- [DeepL Translator](https://www.deepl.com/translator) - URL パラメータでテキストを渡せる
- [Espanso](https://github.com/espanso/espanso) - テキスト展開は別ツールに委譲（[docs/snippets-research.md](snippets-research.md) 参照）
