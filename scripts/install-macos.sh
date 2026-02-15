#!/usr/bin/env bash
set -euo pipefail

# muhenkan-switch-rs インストールスクリプト (macOS)
#
# companion, config.toml, muhenkan-macos.kbd をインストールし、
# kanata を GitHub からダウンロードします。

# ── 未検証警告 ──
echo ""
echo "================================================================"
echo "  WARNING: macOS サポートは未検証です"
echo "  このスクリプトは macOS 環境でテストされていません。"
echo "  問題が発生した場合は Issue で報告してください。"
echo "================================================================"
echo ""

read -rp "続行しますか？ (y/N): " proceed
if [ "$proceed" != "y" ] && [ "$proceed" != "Y" ]; then
    echo "インストールを中止しました。"
    exit 0
fi

# ── 設定 ──
KANATA_VERSION="v1.11.0"
INSTALL_DIR="$HOME/Library/Application Support/muhenkan-switch-rs"
BIN_DIR="$HOME/.local/bin"
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_NAME="com.muhenkan-switch-rs.kanata.plist"
LOG_DIR="$HOME/Library/Logs/muhenkan-switch-rs"

# アーキテクチャ判定
ARCH=$(uname -m)
case "$ARCH" in
    arm64)
        KANATA_ASSET="macos-binaries-arm64.zip"
        KANATA_BINARY="kanata_macos_cmd_allowed_arm64"
        ;;
    x86_64)
        KANATA_ASSET="macos-binaries-x64.zip"
        KANATA_BINARY="kanata_macos_cmd_allowed_x64"
        ;;
    *)
        echo "[ERROR] 未対応のアーキテクチャです: $ARCH"
        exit 1
        ;;
esac

# スクリプトのあるディレクトリ
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== muhenkan-switch-rs インストーラー (macOS) ==="
echo ""
echo "アーキテクチャ: $ARCH"
echo "インストール先: $INSTALL_DIR"
echo "シンボリックリンク: $BIN_DIR"
echo ""

# ── インストールディレクトリ作成 ──
mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"

# ── config.toml のバックアップ ──
if [ -f "$INSTALL_DIR/config.toml" ]; then
    backup_name="config.toml.backup.$(date +%Y%m%d%H%M%S)"
    cp "$INSTALL_DIR/config.toml" "$INSTALL_DIR/$backup_name"
    echo "[OK] 既存の config.toml をバックアップしました: $backup_name"
fi

# ── ファイルコピー ──
copy_file() {
    local src="$SCRIPT_DIR/$1"
    local dest="$INSTALL_DIR/$2"
    if [ -f "$src" ]; then
        cp "$src" "$dest"
        chmod +x "$dest" 2>/dev/null || true
        echo "[OK] $1 -> $2 をコピーしました"
    else
        echo "[SKIP] $1 が見つかりません"
    fi
}

copy_file "companion" "companion"
copy_file "config.toml" "config.toml"
copy_file "muhenkan-macos.kbd" "muhenkan-macos.kbd"

# companion に実行権限を付与
chmod +x "$INSTALL_DIR/companion" 2>/dev/null || true

# ── kanata ダウンロード ──
kanata_dest="$INSTALL_DIR/kanata_cmd_allowed"
if [ -f "$kanata_dest" ]; then
    echo "[SKIP] kanata_cmd_allowed は既にインストール済みです"
    echo "       再ダウンロードする場合は削除してから再実行してください"
else
    echo ""
    echo "kanata $KANATA_VERSION ($ARCH) をダウンロードしています..."

    download_url="https://github.com/jtroo/kanata/releases/download/$KANATA_VERSION/$KANATA_ASSET"
    temp_dir=$(mktemp -d)

    if curl -fSL -o "$temp_dir/kanata.zip" "$download_url"; then
        echo "[OK] ダウンロード完了"

        # 展開
        unzip -q -o "$temp_dir/kanata.zip" -d "$temp_dir/extract"

        # バイナリを探す
        kanata_file=$(find "$temp_dir/extract" -name "$KANATA_BINARY" -type f | head -1)
        if [ -n "$kanata_file" ]; then
            cp "$kanata_file" "$kanata_dest"
            chmod +x "$kanata_dest"
            echo "[OK] kanata_cmd_allowed をインストールしました"
        else
            echo "[ERROR] kanata バイナリが見つかりませんでした: $KANATA_BINARY"
            echo "        手動でダウンロードしてください: https://github.com/jtroo/kanata/releases"
        fi
    else
        echo "[ERROR] kanata のダウンロードに失敗しました"
        echo "        手動でダウンロードしてください: https://github.com/jtroo/kanata/releases"
    fi

    rm -rf "$temp_dir"
fi

# ── シンボリックリンク作成 ──
create_symlink() {
    local target="$1"
    local link_name="$2"
    local link_path="$BIN_DIR/$link_name"

    if [ -f "$target" ]; then
        ln -sf "$target" "$link_path"
        echo "[OK] シンボリックリンク作成: $link_name -> $target"
    fi
}

create_symlink "$INSTALL_DIR/companion" "companion"
create_symlink "$INSTALL_DIR/kanata_cmd_allowed" "kanata_cmd_allowed"

# ── PATH チェック ──
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
    echo ""
    echo "[WARNING] $BIN_DIR が PATH に含まれていません"
    echo "          以下をシェルの設定ファイルに追加してください:"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

# ── launchd エージェント（オプション）──
echo ""
read -rp "launchd エージェント（自動起動）をインストールしますか？ (y/N): " install_agent
if [ "$install_agent" = "y" ] || [ "$install_agent" = "Y" ]; then
    mkdir -p "$PLIST_DIR"
    mkdir -p "$LOG_DIR"

    cat > "$PLIST_DIR/$PLIST_NAME" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.muhenkan-switch-rs.kanata</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/kanata_cmd_allowed</string>
        <string>--cfg</string>
        <string>$INSTALL_DIR/muhenkan-macos.kbd</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/kanata.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/kanata.stderr.log</string>
</dict>
</plist>
EOF

    echo "[OK] launchd エージェントをインストールしました"
    echo "     起動: launchctl load $PLIST_DIR/$PLIST_NAME"
    echo "     停止: launchctl unload $PLIST_DIR/$PLIST_NAME"
    echo ""
    echo "     ※ kanata は sudo での実行が必要な場合があります。"
    echo "       その場合は launchd ではなく手動で起動してください:"
    echo "       sudo kanata_cmd_allowed --cfg \"$INSTALL_DIR/muhenkan-macos.kbd\""
fi

# ── macOS 固有の注意 ──
echo ""
echo "── macOS での注意事項 ──"
echo ""
echo "kanata を macOS で使用するには以下が必要です:"
echo "  1. Karabiner-VirtualHIDDevice のインストール"
echo "     https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice"
echo "  2. sudo での実行（初回）"
echo "     sudo kanata_cmd_allowed --cfg \"$INSTALL_DIR/muhenkan-macos.kbd\""
echo ""
echo "詳細は kanata のリリースページを参照してください:"
echo "  https://github.com/jtroo/kanata/releases"

# ── 完了 ──
echo ""
echo "=== インストール完了 ==="
echo ""
echo "インストール先: $INSTALL_DIR"
echo ""
echo "使い方:"
echo "  1. ターミナルを再起動してください（PATH の反映）"
echo "  2. 以下のコマンドで起動:"
echo "     sudo kanata_cmd_allowed --cfg \"$INSTALL_DIR/muhenkan-macos.kbd\""
echo ""
echo "アンインストール: uninstall-macos.sh を実行してください"
echo ""
