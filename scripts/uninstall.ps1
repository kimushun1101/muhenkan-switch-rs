#Requires -Version 5.1
<#
.SYNOPSIS
    muhenkan-switch-rs アンインストールスクリプト (Windows)
.DESCRIPTION
    kanata プロセスの停止、PATH からの削除、スタートアップショートカット削除、
    インストールディレクトリの削除を行います。
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$INSTALL_DIR = Join-Path $env:LOCALAPPDATA "muhenkan-switch-rs"

Write-Host ""
Write-Host "=== muhenkan-switch-rs アンインストーラー (Windows) ===" -ForegroundColor Cyan
Write-Host ""

if (-not (Test-Path $INSTALL_DIR)) {
    Write-Host "インストールディレクトリが見つかりません: $INSTALL_DIR" -ForegroundColor Yellow
    Write-Host "既にアンインストール済みか、別の場所にインストールされています。"
    exit 0
}

Write-Host "以下を削除します:"
Write-Host "  - インストールディレクトリ: $INSTALL_DIR"
Write-Host "  - ユーザー PATH からインストールディレクトリを除去"
Write-Host "  - スタートアップショートカット（存在する場合）"
Write-Host ""

$confirm = Read-Host "続行しますか？ (y/N)"
if ($confirm -ne "y" -and $confirm -ne "Y") {
    Write-Host "アンインストールを中止しました。" -ForegroundColor Yellow
    exit 0
}

# ── GUI プロセスを停止 ──
Write-Host ""
$guiProcesses = Get-Process -Name "muhenkan-switch-gui" -ErrorAction SilentlyContinue
if ($guiProcesses) {
    Write-Host "muhenkan-switch-gui プロセスを停止しています..." -ForegroundColor Yellow
    $guiProcesses | Stop-Process -Force
    Start-Sleep -Seconds 1
    Write-Host "[OK] muhenkan-switch-gui プロセスを停止しました" -ForegroundColor Green
} else {
    Write-Host "[SKIP] muhenkan-switch-gui プロセスは実行されていません" -ForegroundColor Yellow
}

# ── kanata プロセスを停止 ──
$kanataProcesses = Get-Process -Name "kanata_cmd_allowed" -ErrorAction SilentlyContinue
if ($kanataProcesses) {
    Write-Host "kanata プロセスを停止しています..." -ForegroundColor Yellow
    $kanataProcesses | Stop-Process -Force
    Start-Sleep -Seconds 1
    Write-Host "[OK] kanata プロセスを停止しました" -ForegroundColor Green
} else {
    Write-Host "[SKIP] kanata プロセスは実行されていません" -ForegroundColor Yellow
}

# ── スタートアップショートカット削除 ──
$startupDir = [Environment]::GetFolderPath("Startup")
$shortcutDeleted = $false

# 新しい GUI ショートカット
$guiShortcutPath = Join-Path $startupDir "muhenkan-switch-gui.lnk"
if (Test-Path $guiShortcutPath) {
    Remove-Item $guiShortcutPath -Force
    Write-Host "[OK] スタートアップショートカットを削除しました: muhenkan-switch-gui.lnk" -ForegroundColor Green
    $shortcutDeleted = $true
}

# 旧ショートカット（互換性）
$oldShortcutPath = Join-Path $startupDir "kanata_cmd_allowed.lnk"
if (Test-Path $oldShortcutPath) {
    Remove-Item $oldShortcutPath -Force
    Write-Host "[OK] 旧スタートアップショートカットを削除しました: kanata_cmd_allowed.lnk" -ForegroundColor Green
    $shortcutDeleted = $true
}

if (-not $shortcutDeleted) {
    Write-Host "[SKIP] スタートアップショートカットは存在しません" -ForegroundColor Yellow
}

# ── PATH から削除 ──
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$pathEntries = $userPath -split ";" | Where-Object { $_ -ne "" }

if ($pathEntries -contains $INSTALL_DIR) {
    $newEntries = $pathEntries | Where-Object { $_ -ne $INSTALL_DIR }
    $newPath = $newEntries -join ";"
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    Write-Host "[OK] ユーザー PATH からインストールディレクトリを削除しました" -ForegroundColor Green
} else {
    Write-Host "[SKIP] PATH にインストールディレクトリは含まれていません" -ForegroundColor Yellow
}

# ── インストールディレクトリ削除 ──
try {
    Remove-Item $INSTALL_DIR -Recurse -Force
    Write-Host "[OK] インストールディレクトリを削除しました" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] ディレクトリの削除に失敗しました: $_" -ForegroundColor Red
    Write-Host "        手動で削除してください: $INSTALL_DIR" -ForegroundColor Red
}

# ── 完了 ──
Write-Host ""
Write-Host "=== アンインストール完了 ===" -ForegroundColor Green
Write-Host ""
Write-Host "※ PATH の変更を反映するにはターミナルを再起動してください。"
Write-Host ""
