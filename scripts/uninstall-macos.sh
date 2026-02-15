#!/usr/bin/env bash
set -euo pipefail

# muhenkan-switch-rs アンインストールスクリプト (macOS)

INSTALL_DIR="$HOME/Library/Application Support/muhenkan-switch-rs"
BIN_DIR="$HOME/.local/bin"
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_NAME="com.muhenkan-switch-rs.kanata.plist"
LOG_DIR="$HOME/Library/Logs/muhenkan-switch-rs"

echo ""
echo "=== muhenkan-switch-rs アンインストーラー (macOS) ==="
echo ""

if [ ! -d "$INSTALL_DIR" ]; then
    echo "インストールディレクトリが見つかりません: $INSTALL_DIR"
    echo "既にアンインストール済みか、別の場所にインストールされています。"
    exit 0
fi

echo "以下を削除します:"
echo "  - インストールディレクトリ: $INSTALL_DIR"
echo "  - シンボリックリンク: $BIN_DIR/companion, $BIN_DIR/kanata_cmd_allowed"
if [ -f "$PLIST_DIR/$PLIST_NAME" ]; then
    echo "  - launchd エージェント: $PLIST_NAME"
fi
if [ -d "$LOG_DIR" ]; then
    echo "  - ログディレクトリ: $LOG_DIR"
fi
echo ""

read -rp "続行しますか？ (y/N): " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "アンインストールを中止しました。"
    exit 0
fi

# ── launchd エージェント停止・削除 ──
plist_path="$PLIST_DIR/$PLIST_NAME"
if [ -f "$plist_path" ]; then
    echo ""
    echo "launchd エージェントを停止・削除しています..."
    launchctl unload "$plist_path" 2>/dev/null || true
    rm -f "$plist_path"
    echo "[OK] launchd エージェントを削除しました"
else
    echo "[SKIP] launchd エージェントは存在しません"
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

remove_symlink "companion"
remove_symlink "kanata_cmd_allowed"

# ── インストールディレクトリ削除 ──
rm -rf "$INSTALL_DIR"
echo "[OK] インストールディレクトリを削除しました"

# ── ログディレクトリ削除 ──
if [ -d "$LOG_DIR" ]; then
    rm -rf "$LOG_DIR"
    echo "[OK] ログディレクトリを削除しました"
fi

# ── 完了 ──
echo ""
echo "=== アンインストール完了 ==="
echo ""
