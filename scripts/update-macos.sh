#!/usr/bin/env bash
set -euo pipefail

# muhenkan-switch-rs アップデートスクリプト (macOS)
#
# GitHub Releases から最新版をダウンロードし、install-macos.sh を実行して更新します。

# ── 設定 ──
REPO="kimushun1101/muhenkan-switch-rs"

# アーキテクチャ判定
ARCH=$(uname -m)
case "$ARCH" in
    arm64)
        ASSET_NAME="muhenkan-switch-rs-macos-arm64.tar.gz"
        ;;
    x86_64)
        ASSET_NAME="muhenkan-switch-rs-macos-x64.tar.gz"
        ;;
    *)
        echo "[ERROR] 未対応のアーキテクチャです: $ARCH"
        exit 1
        ;;
esac

echo ""
echo "=== muhenkan-switch-rs アップデーター (macOS) ==="
echo ""
echo "アーキテクチャ: $ARCH"

# ── 最新バージョンを取得 ──
echo ""
echo "最新バージョンを確認しています..."

api_response=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest")

# jq なしで tag_name を抽出
latest_tag=$(echo "$api_response" | grep -o '"tag_name"\s*:\s*"[^"]*"' | sed 's/"tag_name"\s*:\s*"\(.*\)"/\1/')

if [ -z "$latest_tag" ]; then
    echo "[ERROR] 最新バージョンの取得に失敗しました"
    echo "        ネットワーク接続を確認してください。"
    exit 1
fi

# ── 現在のバージョンを取得 ──
current_version="(不明)"
if command -v muhenkan-switch-core &>/dev/null; then
    version_output=$(muhenkan-switch-core --version 2>/dev/null || true)
    if [ -n "$version_output" ]; then
        # "muhenkan-switch-core x.y.z" → "vx.y.z"
        version_string=$(echo "$version_output" | sed 's/^muhenkan-switch-core *//')
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

if ! curl -fSL -o "$temp_dir/archive.tar.gz" "$download_url"; then
    echo "[ERROR] ダウンロードに失敗しました"
    rm -rf "$temp_dir"
    exit 1
fi
echo "[OK] ダウンロード完了"

# ── 展開 ──
tar xzf "$temp_dir/archive.tar.gz" -C "$temp_dir"
echo "[OK] 展開完了"

# ── install-macos.sh を実行 ──
install_script=$(find "$temp_dir" -name "install-macos.sh" -type f | head -1)
if [ -n "$install_script" ]; then
    echo ""
    echo "インストールスクリプトを実行しています..."
    chmod +x "$install_script"
    bash "$install_script"
else
    echo "[ERROR] install-macos.sh が見つかりませんでした"
    rm -rf "$temp_dir"
    exit 1
fi

# ── クリーンアップ ──
rm -rf "$temp_dir"

echo ""
echo "=== アップデート完了 ==="
echo ""
