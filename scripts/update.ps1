#Requires -Version 5.1
<#
.SYNOPSIS
    muhenkan-switch-rs アップデートスクリプト (Windows)
.DESCRIPTION
    GitHub Releases から最新版をダウンロードし、install.ps1 を実行して更新します。
.NOTES
    管理者権限は不要です。
    PowerShell で実行: .\update.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── 設定 ──
$REPO = "kimushun1101/muhenkan-switch-rs"
$ASSET_NAME = "muhenkan-switch-rs-windows-x64.zip"

Write-Host ""
Write-Host "=== muhenkan-switch-rs アップデーター (Windows) ===" -ForegroundColor Cyan
Write-Host ""

# ── TLS 1.2 を有効化 ──
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# ── 最新バージョンを取得 ──
Write-Host "最新バージョンを確認しています..."
try {
    $releaseInfo = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest" -UseBasicParsing
    $latestTag = $releaseInfo.tag_name
} catch {
    Write-Host "[ERROR] 最新バージョンの取得に失敗しました: $_" -ForegroundColor Red
    Write-Host "        ネットワーク接続を確認してください。" -ForegroundColor Red
    exit 1
}

# ── 現在のバージョンを取得 ──
$currentVersion = "(不明)"
try {
    $versionOutput = & muhenkan-switch --version 2>&1
    if ($LASTEXITCODE -eq 0 -and $versionOutput) {
        # "muhenkan-switch x.y.z" → "vx.y.z"
        $versionString = ($versionOutput -replace "^muhenkan-switch\s+", "").Trim()
        $currentVersion = "v$versionString"
    }
} catch {
    # muhenkan-switch が PATH にない場合は無視
}

# ── バージョン表示 ──
Write-Host ""
Write-Host "  現在のバージョン: $currentVersion"
Write-Host "  最新のバージョン: $latestTag"
Write-Host ""

if ($currentVersion -eq $latestTag) {
    Write-Host "既に最新バージョンです。" -ForegroundColor Green
    exit 0
}

# ── 更新確認 ──
$confirm = Read-Host "更新しますか？ (y/N)"
if ($confirm -ne "y" -and $confirm -ne "Y") {
    Write-Host "更新を中止しました。"
    exit 0
}

# ── ダウンロード ──
Write-Host ""
Write-Host "$latestTag をダウンロードしています..." -ForegroundColor Cyan

$downloadUrl = "https://github.com/$REPO/releases/download/$latestTag/$ASSET_NAME"
$tempZip = Join-Path $env:TEMP "muhenkan-switch-rs-update.zip"
$tempExtract = Join-Path $env:TEMP "muhenkan-switch-rs-update"

try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $tempZip -UseBasicParsing
    Write-Host "[OK] ダウンロード完了" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] ダウンロードに失敗しました: $_" -ForegroundColor Red
    exit 1
}

# ── 展開 ──
try {
    if (Test-Path $tempExtract) {
        Remove-Item $tempExtract -Recurse -Force
    }
    Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force
    Write-Host "[OK] 展開完了" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] 展開に失敗しました: $_" -ForegroundColor Red
    exit 1
}

# ── install.ps1 を実行 ──
$installScript = Get-ChildItem -Path $tempExtract -Recurse -Filter "install.ps1" | Select-Object -First 1
if ($installScript) {
    Write-Host ""
    Write-Host "インストールスクリプトを実行しています..." -ForegroundColor Cyan
    & powershell.exe -ExecutionPolicy Bypass -File $installScript.FullName
} else {
    Write-Host "[ERROR] install.ps1 が見つかりませんでした" -ForegroundColor Red
    exit 1
}

# ── クリーンアップ ──
if (Test-Path $tempZip) { Remove-Item $tempZip -Force -ErrorAction SilentlyContinue }
if (Test-Path $tempExtract) { Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue }

Write-Host ""
Write-Host "=== アップデート完了 ===" -ForegroundColor Green
Write-Host ""
