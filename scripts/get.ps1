#Requires -Version 5.1
<#
.SYNOPSIS
    muhenkan-switch-rs ワンライナーインストーラー (Windows)
.DESCRIPTION
    GitHub Releases から最新版をダウンロードし、install.ps1 を実行します。
.NOTES
    使い方:
    irm https://raw.githubusercontent.com/kimushun1101/muhenkan-switch-rs/main/scripts/get.ps1 | iex
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── 設定 ──
$REPO = "kimushun1101/muhenkan-switch-rs"
$ASSET_NAME = "muhenkan-switch-rs-windows-x64.zip"

Write-Host ""
Write-Host "=== muhenkan-switch-rs インストーラー (Windows) ===" -ForegroundColor Cyan
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

Write-Host "最新バージョン: $latestTag"

# ── ダウンロード ──
Write-Host ""
Write-Host "$latestTag をダウンロードしています..." -ForegroundColor Cyan

$downloadUrl = "https://github.com/$REPO/releases/download/$latestTag/$ASSET_NAME"
$tempZip = Join-Path $env:TEMP "muhenkan-switch-rs-install.zip"
$tempExtract = Join-Path $env:TEMP "muhenkan-switch-rs-install"

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
Write-Host "=== インストール完了 ===" -ForegroundColor Green
Write-Host ""
