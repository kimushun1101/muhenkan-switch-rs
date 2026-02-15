#!/usr/bin/env bash
set -euo pipefail

# muhenkan-switch-rs ワンライナーインストーラー
#
# 使い方:
#   curl -fsSL https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.sh | sh
#
# パイプ実行時は一時ファイルに保存してから bash で実行するため、
# install スクリプト内の read プロンプトも正常に動作します。

# ── パイプ実行ガード ──
# stdin がパイプの場合、スクリプト全体を一時ファイルに書き出して再実行する
if [ ! -t 0 ]; then
    tmp_script=$(mktemp)
    cat > "$tmp_script"
    exec bash "$tmp_script" "$@"
    # exec で置き換わるため、ここには到達しない
fi

# ── 設定 ──
REPO="kimushun1101/muhenkan-switch-rs"

echo ""
echo "=== muhenkan-switch-rs インストーラー ==="
echo ""

# ── OS・アーキテクチャ検出 ──
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64) ASSET_NAME="muhenkan-switch-rs-linux-x64.tar.gz" ;;
            *) echo "[ERROR] 未対応のアーキテクチャです: $ARCH"; exit 1 ;;
        esac
        INSTALL_SCRIPT="install.sh"
        echo "OS: Linux ($ARCH)"
        ;;
    Darwin)
        case "$ARCH" in
            arm64)  ASSET_NAME="muhenkan-switch-rs-macos-arm64.tar.gz" ;;
            x86_64) ASSET_NAME="muhenkan-switch-rs-macos-x64.tar.gz" ;;
            *) echo "[ERROR] 未対応のアーキテクチャです: $ARCH"; exit 1 ;;
        esac
        INSTALL_SCRIPT="install-macos.sh"
        echo "OS: macOS ($ARCH)"
        ;;
    *)
        echo "[ERROR] 未対応の OS です: $OS"
        echo "        Windows の場合は PowerShell で以下を実行してください:"
        echo "        irm https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.ps1 | iex"
        exit 1
        ;;
esac

# ── 最新バージョンを取得 ──
echo ""
echo "最新バージョンを確認しています..."

if command -v curl &>/dev/null; then
    api_response=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")
elif command -v wget &>/dev/null; then
    api_response=$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest")
else
    echo "[ERROR] curl または wget が必要です"
    exit 1
fi

# jq なしで tag_name を抽出
latest_tag=$(echo "$api_response" | grep -o '"tag_name"\s*:\s*"[^"]*"' | sed 's/"tag_name"\s*:\s*"\(.*\)"/\1/')

if [ -z "$latest_tag" ]; then
    echo "[ERROR] 最新バージョンの取得に失敗しました"
    echo "        ネットワーク接続を確認してください。"
    exit 1
fi

echo "最新バージョン: $latest_tag"

# ── ダウンロード ──
echo ""
echo "$latest_tag をダウンロードしています..."

download_url="https://github.com/$REPO/releases/download/$latest_tag/$ASSET_NAME"
temp_dir=$(mktemp -d)

if command -v curl &>/dev/null; then
    downloader="curl -fSL -o"
else
    downloader="wget -q -O"
fi

if ! $downloader "$temp_dir/archive.tar.gz" "$download_url"; then
    echo "[ERROR] ダウンロードに失敗しました"
    rm -rf "$temp_dir"
    exit 1
fi
echo "[OK] ダウンロード完了"

# ── 展開 ──
tar xzf "$temp_dir/archive.tar.gz" -C "$temp_dir"
echo "[OK] 展開完了"

# ── インストールスクリプトを実行 ──
install_script=$(find "$temp_dir" -name "$INSTALL_SCRIPT" -type f | head -1)
if [ -n "$install_script" ]; then
    echo ""
    echo "インストールスクリプトを実行しています..."
    chmod +x "$install_script"
    bash "$install_script"
else
    echo "[ERROR] $INSTALL_SCRIPT が見つかりませんでした"
    rm -rf "$temp_dir"
    exit 1
fi

# ── クリーンアップ ──
rm -rf "$temp_dir"

echo ""
echo "=== インストール完了 ==="
echo ""
