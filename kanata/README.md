# kanata 設定ファイル

## ファイル一覧

| ファイル | 対象OS | 状態 |
|---------|--------|------|
| `muhenkan.kbd` | Windows / Linux | 検証済み |
| `muhenkan-macos.kbd` | macOS | ⚠️ 未検証 |

## kanata のインストール

[kanata リリースページ](https://github.com/jtroo/kanata/releases) から **`kanata_cmd_allowed`** 版をダウンロードしてください。

`cmd` アクション（muhenkan-switch バイナリの呼び出し）を使用するため、通常版（`kanata`）ではなく `cmd_allowed` 版が必要です。

## 起動方法

```bash
# Windows
kanata_cmd_allowed.exe --cfg muhenkan.kbd

# Linux
kanata --cfg muhenkan.kbd

# macOS (未検証、sudo 必要)
sudo kanata --cfg muhenkan-macos.kbd
```

## カスタマイズ

### tap-hold のタイミング調整

```lisp
;; デフォルト: 200ms
(defalias
  mh (tap-hold 200 200 muhenkan (layer-while-held mh-layer))
)
```

最初の `200` が tap のタイムアウト、次の `200` が hold の判定時間です。
短くすると反応が速くなりますが、誤判定が増えます。

### キーマッピングの追加・変更

`defsrc` にキーを追加し、`deflayer` の対応位置にアクションを記述してください。
kanata の設定ガイドは [こちら](https://github.com/jtroo/kanata/wiki/Configuration-guide)。
