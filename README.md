# muhenkan-switch-rs

無変換キーと同時押しを起点としたクロスプラットフォーム・ショートカットツール。

[muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch)（AutoHotkey版）を
[kanata](https://github.com/jtroo/kanata) + Rust製companionバイナリ で再実装したものです。

## 対応環境

| OS | 対応状況 | 備考 |
|----|----------|------|
| Windows 10/11 | ✅ 検証済み | |
| Linux (X11/Wayland) | ✅ 検証済み | evdev 対応ディストリビューション |
| macOS | ⚠️ 未検証 | JIS配列Mac向け設定ファイルを同梱。動作報告歓迎 |

**日本語キーボード（JIS配列）が必須です。** US配列には対応していません。

## 機能

無変換キーを押しながら他のキーを押すことで、以下の操作ができます。

- **Vim風カーソル移動**: H/J/K/L → ←/↓/↑/→
- **単語・行頭行末移動**: U/I → 単語移動、Y/O → Home/End
- **削除**: N → BackSpace、M → Delete
- **アプリ切り替え**: A/W/E/S/D/F → 指定アプリを最前面に
- **Web検索**: Q/R/T/G → 選択テキストで辞書・翻訳・検索
- **フォルダオープン**: 1/2/3/4/5 → ドキュメント/ダウンロード等
- **タイムスタンプ**: V/C/X → タイムスタンプの貼り付け・コピー・切り取り
- **句読点入力**: カンマ → 「、」、ピリオド → 「。」

詳細は [docs/DESIGN.md](docs/DESIGN.md) を参照してください。

## セットアップ

### 1. ダウンロード

[Releases](https://github.com/kimushun1101/muhenkan-switch-rs/releases) から
お使いの OS 用の zip をダウンロードし、任意のフォルダに展開してください。

zip の中身:
```
muhenkan-switch-rs/
├── kanata_cmd_allowed(.exe)   # kanata 本体
├── companion(.exe)            # companion ツール
├── muhenkan.kbd               # kanata 設定ファイル
└── config.toml                # companion 設定ファイル
```

### 2. kanata のインストール

zip に kanata が同梱されていない場合は、
[kanata リリースページ](https://github.com/jtroo/kanata/releases) から
**`kanata_cmd_allowed`** 版をダウンロードしてください（`cmd` アクション有効版が必要です）。

#### Linux の追加設定

sudo なしで実行するため、以下のグループ設定が必要です:

```bash
sudo groupadd -f uinput
sudo usermod -aG input $USER
sudo usermod -aG uinput $USER

echo 'KERNEL=="uinput", MODE="0660", GROUP="uinput", OPTIONS+="static_node=uinput"' \
  | sudo tee /etc/udev/rules.d/99-uinput.rules

sudo udevadm control --reload-rules && sudo udevadm trigger
# 再ログインが必要
```

### 3. 起動

```bash
# Windows
kanata_cmd_allowed.exe --cfg muhenkan.kbd

# Linux
kanata --cfg muhenkan.kbd
```

無変換キーを押しながら H/J/K/L でカーソルが移動すれば成功です。
`Ctrl+Space+Esc` で kanata を終了できます。

### 4. 常駐化（オプション）

#### Windows — スタートアップ登録

`Win+R` → `shell:startup` → kanata のショートカットを配置。
または [kanata-tray](https://github.com/rszyma/kanata-tray) を使用。

#### Linux — systemd

```bash
mkdir -p ~/.config/systemd/user

cat << 'EOF' > ~/.config/systemd/user/kanata.service
[Unit]
Description=Kanata keyboard remapper

[Service]
ExecStart=%h/muhenkan-switch-rs/kanata --cfg %h/muhenkan-switch-rs/muhenkan.kbd
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now kanata.service
```

## macOS をお使いの方へ

macOS 用の設定ファイル (`muhenkan-macos.kbd`) を同梱していますが、
開発者の検証環境がないため **動作未検証** です。
JIS配列 Mac での「英数」キーが kanata 上で `eisu` として認識される前提で
作成しています。動作報告や修正 PR を歓迎します。

macOS では [Karabiner-VirtualHIDDevice](https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice)
のインストールと `sudo` 実行が必要です。
詳細は [kanata リリースページ](https://github.com/jtroo/kanata/releases) の macOS 手順を参照してください。

## カスタマイズ

### キーマッピングの変更

`muhenkan.kbd` を編集してください。
kanata の設定ガイドは [こちら](https://github.com/jtroo/kanata/wiki/Configuration-guide)。

### companion の設定変更

`config.toml` で検索エンジンのURL、アプリ名、フォルダパス等を変更できます。

## 開発

```bash
# Rust ツールチェーンのインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# ビルド
cd companion
cargo build --release
```

## ライセンス

GPL-2.0 — [muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch) を継承。

## 旧版（AutoHotkey版）

Windows 専用の AutoHotkey 版は [muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch) にあります。
