---
title: Snippets（テキスト展開）機能の調査
date: 2026-02-21
status: 見送り（Espanso 等の既存ツールを推奨）
tags:
  - snippets
  - text-expander
  - hotstring
  - espanso
---

## 概要

`;date` や `;mail` のようなトリガー文字列を入力すると、定型テキストに自動展開される機能（Hotstring / Text Expander）の実装可否を調査した。

**結論: 本プロジェクトでは実装しない。** Espanso 等の成熟した既存ツールとの併用を推奨する。

---

## 想定仕様

### トリガーの仕組み

1. ユーザーが `;date` と入力
2. バックグラウンドプロセスがキー入力を監視し、パターン `;date` を検出
3. 入力済みの `;date`（5文字）をバックスペースで削除
4. 展開テキスト（例: `2026-02-21`）をペーストまたはキー入力で挿入

`;`（セミコロン）をプレフィックスに使う理由: 文頭にほぼ出現しないため、通常の入力と誤検出しにくい。

### 設定例

```toml
[snippets]
date  = {trigger = ";date",  text = "{%Y-%m-%d}"}
time  = {trigger = ";time",  text = "{%H:%M}"}
mail  = {trigger = ";mail",  text = "taro.yamada@example.com"}
addr  = {trigger = ";addr",  text = "〒100-0001 東京都千代田区千代田1-1"}
greet = {trigger = ";greet", text = "お疲れ様です。○○部の山田です。"}
close = {trigger = ";close", text = "ご確認のほどよろしくお願いいたします。"}
mtg   = {trigger = ";mtg",   text = "## 議題\n- \n## 決定事項\n- \n## TODO\n-"}
sig   = {trigger = ";sig",   text = "よろしくお願いいたします。\n山田太郎"}
```

---

## 現在のアーキテクチャとの適合性

### muhenkan-switch の実行モデル

```
ユーザー → kanata（キー入力監視）→ cmd で muhenkan-switch-core を起動 → 実行 → 終了
```

- kanata が全てのキー入力をインターセプトし、無変換+キーの組み合わせのみを検出
- muhenkan-switch-core は**毎回起動→終了するステートレスな CLI** であり、常駐プロセスではない
- テキスト出力は clipboard + Ctrl+V（SendInput）による貼り付け

### スニペット実装に必要な要素

| 要素 | 現状 | 必要な変更 | 難易度 |
|------|------|-----------|--------|
| 全キー入力の監視 | kanata が担当（無変換+キーのみ検出） | 全キーストロークをストリームで受け取る仕組み | 大 |
| 入力シーケンスの状態管理 | なし（ステートレス CLI） | 常駐デーモン化してキー入力バッファを保持 | 大 |
| バックスペース送信 | 未実装（SendInput で技術的には可能） | 追加実装 | 小 |
| テキスト出力 | clipboard + Ctrl+V（timestamp で実績あり） | そのまま流用可能 | なし |

### 2つの実装アプローチ

#### A. kanata 拡張方式

- kanata 側でキーシーケンス検出機能を追加し、マッチ時に `cmd muhenkan-switch-core snippet <name>` を呼ぶ
- muhenkan-switch-core はバックスペース送信 + テキスト展開のみ担当
- **課題:** kanata は「キーリマッパー」であり、文字列シーケンス検出は設計思想の外。kanata 本体への機能提案が必要

#### B. 独立デーモン方式

- muhenkan-switch-core を常駐プロセス化し、独自にキーボードフックを追加
- Windows: `SetWindowsHookEx`、Linux: `/dev/input`、macOS: `CGEventTap`
- **課題:** kanata と二重にキーボードフックすることになり、競合やパフォーマンスの懸念がある

---

## 既存ツール: Espanso

[Espanso](https://github.com/espanso/espanso) は Rust 製のクロスプラットフォームテキストエキスパンダーで、本機能の完全な上位互換である。

### Espanso の特徴

- **クロスプラットフォーム:** Windows / macOS / Linux 対応
- **トリガー方式:** `:date` `:mail` のようなプレフィックス + キーワード（本調査と同じ仕組み）
- **展開方式:** 短いテキストは Inject（キー入力シミュレーション）、長いテキストは Clipboard（ペースト）
- **動的展開:** 日付・時刻・シェルコマンド出力・クリップボード内容を埋め込み可能
- **正規表現トリガー:** 高度なパターンマッチも可能
- **YAML 設定:** 設定ファイルで管理

### Espanso の設定例

```yaml
matches:
  - trigger: ";date"
    replace: "{{date}}"
    vars:
      - name: date
        type: date
        params:
          format: "%Y-%m-%d"

  - trigger: ";mail"
    replace: "taro.yamada@example.com"

  - trigger: ";greet"
    replace: "お疲れ様です。○○部の山田です。"
```

### muhenkan-switch で実装しない理由

1. **車輪の再発明:** Espanso が既に成熟した実装を提供しており、自前実装のメリットが薄い
2. **アーキテクチャの乖離:** 現在のステートレス CLI モデルとは根本的に異なる常駐型の設計が必要
3. **キーボードフックの競合:** kanata と二重にフックする問題の解決が複雑
4. **スコープの肥大化:** muhenkan-switch の本来の役割（無変換キーによるアプリ切替・検索・フォルダ操作）から逸脱する

---

## 推奨: Espanso との併用

muhenkan-switch と Espanso は競合せず共存できる。

- **muhenkan-switch:** 無変換キー（修飾キー）+ 単一キーによるアクション実行
- **Espanso:** 文字列シーケンスによるテキスト展開

ユーザーがスニペット機能を必要とする場合は、Espanso のインストールと設定を推奨する。

### 参考リンク

- [Espanso 公式サイト](https://espanso.org/)
- [Espanso GitHub リポジトリ](https://github.com/espanso/espanso)
- [Espanso ドキュメント: Matches Basics](https://espanso.org/docs/matches/basics/)
