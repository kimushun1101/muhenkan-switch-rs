#!/usr/bin/env bash
set -euo pipefail

# muhenkan-switch-rs アップデートスクリプト (Linux)
#
# GitHub Releases から最新版をダウンロードし、install.sh を実行して更新します。
# root 権限は不要です。

# ── 設定 ──
REPO="kimushun1101/muhenkan-switch-rs"
ASSET_NAME="muhenkan-switch-rs-linux-x64.tar.gz"

echo ""
echo "=== muhenkan-switch-rs アップデーター (Linux) ==="
echo ""

# ── 最新バージョンを取得 ──
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

# ── 現在のバージョンを取得 ──
current_version="(不明)"
if command -v muhenkan-switch &>/dev/null; then
    version_output=$(muhenkan-switch --version 2>/dev/null || true)
    if [ -n "$version_output" ]; then
        # "muhenkan-switch x.y.z" → "vx.y.z"
        version_string=$(echo "$version_output" | sed 's/^muhenkan-switch\s*//')
        current_version="v$version_string"
    fi
fi

# ── バージョン表示 ──
echo ""
echo "  現在のバージョン: $current_version"
echo "  最新のバージョン: $latest_tag"
echo ""

if [ "$current_version" = "$latest_tag" ]; then
    echo "既に最新バージョンです。"
    exit 0
fi

# ── 更新確認 ──
read -rp "更新しますか？ (y/N): " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "更新を中止しました。"
    exit 0
fi

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

# ── install.sh を実行 ──
install_script=$(find "$temp_dir" -name "install.sh" -type f | head -1)
if [ -n "$install_script" ]; then
    echo ""
    echo "インストールスクリプトを実行しています..."
    chmod +x "$install_script"
    bash "$install_script"
else
    echo "[ERROR] install.sh が見つかりませんでした"
    rm -rf "$temp_dir"
    exit 1
fi

# ── クリーンアップ ──
rm -rf "$temp_dir"

echo ""
echo "=== アップデート完了 ==="
echo ""
