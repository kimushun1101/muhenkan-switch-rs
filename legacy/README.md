# 旧版（AutoHotkey版）からの移行ガイド

## 概要

`muhenkan-switch-rs` は、AutoHotkey 版 [muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch) をクロスプラットフォーム対応で再実装したものです。

## 主な変更点

| 項目 | 旧版（AHK） | 新版（kanata + Rust） |
|------|-------------|---------------------|
| OS対応 | Windows のみ | Windows / Linux / macOS(未検証) |
| キーリマップ | AutoHotkey v2 | kanata (.kbd ファイル) |
| 補助機能 | AHK スクリプト | Rust製 companion バイナリ |
| 設定 | GUI + AHK 設定ファイル | config.toml (テキスト編集) |
| 配布形式 | AHK スクリプト or exe | ネイティブバイナリ |
| ランタイム | AutoHotkey | なし（スタンドアロン） |

## 機能の移行状況

| 機能 | 旧版 | 新版 | 備考 |
|------|------|------|------|
| Vim風カーソル移動 | ✅ | ✅ | kanata レイヤーで実現 |
| アプリ切り替え | ✅ | ✅ | companion switch-app |
| Web検索 | ✅ | ✅ | companion search |
| フォルダオープン | ✅ | ✅ | companion open-folder |
| タイムスタンプ | ✅ | ✅ | companion timestamp |
| 句読点入力 | ✅ | ✅ | kanata unicode |
| スクリーンショット | ✅ | ✅ | companion screenshot |
| ホットストリング | ✅ | ⬜ | Phase 4 で対応予定 |
| GUI設定画面 | ✅ | ⬜ | Phase 4 で対応予定 |
| 自動更新 | ✅ | ⬜ | Phase 4 で対応予定 |

## 移行手順

1. [muhenkan-switch-rs のリリースページ](https://github.com/kimushun1101/muhenkan-switch-rs/releases) からダウンロード
2. `config.toml` を自分の環境に合わせて編集（旧版の設定を手動で転記）
3. kanata を起動して動作確認
4. 問題なければ旧版の AutoHotkey スクリプトを無効化
