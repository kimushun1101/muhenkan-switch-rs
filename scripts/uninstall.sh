#!/usr/bin/env bash
set -euo pipefail

# muhenkan-switch-rs アンインストールスクリプト (Linux)

INSTALL_DIR="$HOME/.local/share/muhenkan-switch-rs"
BIN_DIR="$HOME/.local/bin"
SERVICE_FILE="$HOME/.config/systemd/user/kanata.service"
AUTOSTART_FILE="$HOME/.config/autostart/muhenkan-switch.desktop"

echo ""
echo "=== muhenkan-switch-rs アンインストーラー (Linux) ==="
echo ""

if [ ! -d "$INSTALL_DIR" ]; then
    echo "インストールディレクトリが見つかりません: $INSTALL_DIR"
    echo "既にアンインストール済みか、別の場所にインストールされています。"
    exit 0
fi

echo "以下を削除します:"
echo "  - インストールディレクトリ: $INSTALL_DIR"
echo "  - シンボリックリンク: $BIN_DIR/muhenkan-switch, $BIN_DIR/muhenkan-switch-core, $BIN_DIR/kanata_cmd_allowed"
if [ -f "$SERVICE_FILE" ]; then
    echo "  - systemd サービス: kanata.service"
fi
if [ -f "$AUTOSTART_FILE" ]; then
    echo "  - 自動起動: $AUTOSTART_FILE"
fi
echo ""

read -rp "続行しますか？ (y/N): " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "アンインストールを中止しました。"
    exit 0
fi

# ── systemd サービス停止・削除（互換性）──
if [ -f "$SERVICE_FILE" ]; then
    echo ""
    echo "systemd サービスを停止・削除しています..."
    systemctl --user stop kanata.service 2>/dev/null || true
    systemctl --user disable kanata.service 2>/dev/null || true
    rm -f "$SERVICE_FILE"
    systemctl --user daemon-reload
    echo "[OK] systemd サービスを削除しました"
else
    echo "[SKIP] systemd サービスは存在しません"
fi

# ── 自動起動 .desktop ファイル削除 ──
if [ -f "$AUTOSTART_FILE" ]; then
    rm -f "$AUTOSTART_FILE"
    echo "[OK] 自動起動設定を削除しました"
else
    echo "[SKIP] 自動起動設定は存在しません"
fi

# ── シンボリックリンク削除（安全チェック付き）──
remove_symlink() {
    local link_path="$BIN_DIR/$1"
    if [ -L "$link_path" ]; then
        local target
        target=$(readlink "$link_path")
        # インストールディレクトリを指しているか確認
        if [[ "$target" == "$INSTALL_DIR/"* ]]; then
            rm -f "$link_path"
            echo "[OK] シンボリックリンクを削除しました: $1"
        else
            echo "[SKIP] $1 は muhenkan-switch-rs のリンクではありません (-> $target)"
        fi
    else
        echo "[SKIP] $1 はシンボリックリンクではないか、存在しません"
    fi
}

remove_symlink "muhenkan-switch"
remove_symlink "muhenkan-switch-core"
remove_symlink "muhenkan-switch-gui"
remove_symlink "kanata_cmd_allowed"

# ── インストールディレクトリ削除 ──
rm -rf "$INSTALL_DIR"
echo "[OK] インストールディレクトリを削除しました"

# ── 完了 ──
echo ""
echo "=== アンインストール完了 ==="
echo ""
