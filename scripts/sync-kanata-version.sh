#!/usr/bin/env bash
set -euo pipefail

# kanata バージョンを kanata-version.txt から読み取り、
# 4 つのファイルに反映する同期スクリプト。
#
# 使い方:
#   ./scripts/sync-kanata-version.sh            # kanata-version.txt の値を反映
#   ./scripts/sync-kanata-version.sh v1.12.0    # 指定バージョンで更新（kanata-version.txt も更新）

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION_FILE="$REPO_ROOT/kanata-version.txt"

# macOS (BSD sed) と Linux (GNU sed) の両方で動く sed -i ラッパー
sedi() {
    if sed --version >/dev/null 2>&1; then
        # GNU sed
        sed -i "$@"
    else
        # BSD sed (macOS)
        sed -i '' "$@"
    fi
}

if [ $# -ge 1 ]; then
    # 引数で指定された場合、kanata-version.txt も更新
    NEW_VERSION="$1"
    printf '%s\n' "$NEW_VERSION" > "$VERSION_FILE"
    echo "[sync] kanata-version.txt を $NEW_VERSION に更新しました"
else
    NEW_VERSION="$(tr -d '[:space:]' < "$VERSION_FILE")"
fi

if [ -z "$NEW_VERSION" ]; then
    echo "[ERROR] バージョンが空です" >&2
    exit 1
fi

echo "[sync] kanata バージョンを $NEW_VERSION に同期します..."

# 1. scripts/install.sh
FILE="$REPO_ROOT/scripts/install.sh"
if [ -f "$FILE" ]; then
    sedi "s/^KANATA_VERSION=\"[^\"]*\"/KANATA_VERSION=\"$NEW_VERSION\"/" "$FILE"
    echo "[OK] scripts/install.sh"
fi

# 2. scripts/install.ps1
FILE="$REPO_ROOT/scripts/install.ps1"
if [ -f "$FILE" ]; then
    sedi "s/^\\\$KANATA_VERSION = \"[^\"]*\"/\$KANATA_VERSION = \"$NEW_VERSION\"/" "$FILE"
    echo "[OK] scripts/install.ps1"
fi

# 3. scripts/install-macos.sh
FILE="$REPO_ROOT/scripts/install-macos.sh"
if [ -f "$FILE" ]; then
    sedi "s/^KANATA_VERSION=\"[^\"]*\"/KANATA_VERSION=\"$NEW_VERSION\"/" "$FILE"
    echo "[OK] scripts/install-macos.sh"
fi

# 4. mise.toml (VERSION = "vX.Y.Z" inside fetch-kanata task)
FILE="$REPO_ROOT/mise.toml"
if [ -f "$FILE" ]; then
    sedi "s/^VERSION = \"v[^\"]*\"/VERSION = \"$NEW_VERSION\"/" "$FILE"
    echo "[OK] mise.toml"
fi

echo "[sync] 完了"
