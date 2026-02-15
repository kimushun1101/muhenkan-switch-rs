# muhenkan-switch-rs

無変換キーと同時押しを起点としたクロスプラットフォーム・ショートカットツール。

[muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch)（AutoHotkey版）を
[kanata](https://github.com/jtroo/kanata) + Rust製バイナリ で再実装したものです。

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

### 1. インストール

以下のコマンドをターミナルに貼り付けて実行するだけで、最新版のダウンロードからインストールまで自動で行われます。

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.ps1 | iex
```

> **セキュリティについて**: スクリプトの内容を事前に確認したい場合は、先にダウンロードしてから実行できます。
> ```bash
> # Linux / macOS
> curl -fsSL https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.sh -o get.sh
> less get.sh    # 内容を確認
> bash get.sh    # 実行
> ```
> ```powershell
> # Windows
> irm https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.ps1 -OutFile get.ps1
> Get-Content get.ps1   # 内容を確認
> .\get.ps1             # 実行
> ```

<details>
<summary>手動インストール（アーカイブをダウンロードする方法）</summary>

[Releases](https://github.com/kimushun1101/muhenkan-switch-rs/releases) から
お使いの OS 用のアーカイブをダウンロード・展開し、インストールスクリプトを実行してください。

```
# Windows: install.bat をダブルクリック
# または install.ps1 を右クリック →「PowerShell で実行」

# Linux
./install.sh

# macOS
./install-macos.sh
```

</details>

インストールスクリプトは以下を自動で行います:
- kanata のダウンロード（GitHub Releases から）
- ファイルの配置（下記インストール先）
- PATH の設定（Windows: ユーザー環境変数、Linux/macOS: `~/.local/bin` にシンボリックリンク）
- オプション: 自動起動の設定（Windows: スタートアップ、Linux: systemd、macOS: launchd）

| OS | インストール先 |
|----|--------------|
| Windows | `%LOCALAPPDATA%\muhenkan-switch-rs` |
| Linux | `~/.local/share/muhenkan-switch-rs` |
| macOS | `~/Library/Application Support/muhenkan-switch-rs` |

インストール後のディレクトリ構成:
```
<install_dir>/
├── kanata_cmd_allowed(.exe)   # kanata 本体（自動ダウンロード）
├── muhenkan-switch(.exe)       # muhenkan-switch ツール
├── muhenkan.kbd               # kanata 設定ファイル (macOS: muhenkan-macos.kbd)
└── config.toml                # muhenkan-switch 設定ファイル
```

### 2. ターミナルを再起動

PATH の変更を反映するため、ターミナルを再起動してください。

### 3. 起動

```bash
# Windows
kanata_cmd_allowed.exe --cfg "%LOCALAPPDATA%\muhenkan-switch-rs\muhenkan.kbd"

# Linux
kanata_cmd_allowed --cfg ~/.local/share/muhenkan-switch-rs/muhenkan.kbd

# macOS (sudo が必要)
sudo kanata_cmd_allowed --cfg ~/Library/Application\ Support/muhenkan-switch-rs/muhenkan-macos.kbd
```

無変換キーを押しながら H/J/K/L でカーソルが移動すれば成功です。
`Ctrl+Space+Esc` で kanata を終了できます。

#### Linux の追加設定

sudo なしで実行するため、以下のグループ設定が必要です（インストールスクリプト実行時にも案内されます）:

```bash
sudo groupadd -f uinput
sudo usermod -aG input $USER
sudo usermod -aG uinput $USER

echo 'KERNEL=="uinput", MODE="0660", GROUP="uinput", OPTIONS+="static_node=uinput"' \
  | sudo tee /etc/udev/rules.d/99-uinput.rules

sudo udevadm control --reload-rules && sudo udevadm trigger
# 再ログインが必要
```

### アンインストール

インストール先にあるアンインストールスクリプトを実行してください:

```bash
# Windows（PowerShell）
& "$env:LOCALAPPDATA\muhenkan-switch-rs\uninstall.ps1"

# Linux
~/.local/share/muhenkan-switch-rs/uninstall.sh

# macOS
~/Library/Application\ Support/muhenkan-switch-rs/uninstall-macos.sh
```

手動で削除する場合は、以下を削除してください:
- インストールディレクトリ（上記表を参照）
- PATH からインストールディレクトリを除去（Windows のみ）
- 自動起動設定（Windows: スタートアップショートカット、Linux: systemd サービス、macOS: launchd エージェント）

### 更新

インストール先にある更新スクリプトを実行すると、最新版に更新できます。

```
# Windows（PowerShell）
& "$env:LOCALAPPDATA\muhenkan-switch-rs\update.ps1"

# Linux
~/.local/share/muhenkan-switch-rs/update.sh

# macOS
~/Library/Application\ Support/muhenkan-switch-rs/update-macos.sh
```

更新スクリプトは以下を自動で行います:
- GitHub Releases から最新バージョンの確認
- 現在のバージョンとの比較（既に最新の場合は終了）
- 最新版のダウンロード・展開
- インストールスクリプトの実行（既存インストールを上書き更新）

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

### muhenkan-switch の設定変更

`config.toml` で検索エンジンのURL、アプリ名、フォルダパス等を変更できます。

## 開発

### 前提条件

- [Rust ツールチェーン](https://rustup.rs/)
- [mise](https://mise.jdx.dev/)（タスクランナーとして使用）

```bash
# Rust のインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# mise のインストール (Linux/macOS)
curl https://mise.jdx.dev/install.sh | sh

# mise のインストール (Windows - PowerShell)
# winget install jdx.mise
# または Scoop: scoop install mise
```

### 開発タスク

```bash
mise run build      # debug ビルド → ルートにコピー
mise run release    # release ビルド → ルートにコピー
mise run run        # debug ビルド後、kanata も起動
```

## ライセンス

GPL-2.0 — [muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch) を継承。

## 旧版（AutoHotkey版）

Windows 専用の AutoHotkey 版は [muhenkan-switch](https://github.com/kimushun1101/muhenkan-switch) にあります。
