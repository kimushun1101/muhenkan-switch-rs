#Requires -Version 5.1
<#
.SYNOPSIS
    muhenkan-switch-rs インストールスクリプト (Windows)
.DESCRIPTION
    muhenkan-switch.exe, config.toml, muhenkan.kbd をインストールし、
    kanata を GitHub からダウンロードします。
.NOTES
    管理者権限は不要です。
    PowerShell で実行: .\install.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── 設定 ──
$KANATA_VERSION = "v1.11.0"
$KANATA_ASSET = "windows-binaries-x64.zip"
$KANATA_BINARY = "kanata_windows_gui_winIOv2_cmd_allowed_x64.exe"
$INSTALL_DIR = Join-Path $env:LOCALAPPDATA "muhenkan-switch-rs"

# ── スクリプトのあるディレクトリ（展開した zip のルート）──
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

Write-Host ""
Write-Host "=== muhenkan-switch-rs インストーラー (Windows) ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "インストール先: $INSTALL_DIR"
Write-Host ""

# ── インストールディレクトリ作成 ──
if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
    Write-Host "[OK] インストールディレクトリを作成しました" -ForegroundColor Green
}

# ── config.toml のバックアップ ──
$configDest = Join-Path $INSTALL_DIR "config.toml"
if (Test-Path $configDest) {
    $backupName = "config.toml.backup." + (Get-Date -Format "yyyyMMddHHmmss")
    $backupPath = Join-Path $INSTALL_DIR $backupName
    Copy-Item $configDest $backupPath
    Write-Host "[OK] 既存の config.toml をバックアップしました: $backupName" -ForegroundColor Yellow
}

# ── ファイルコピー ──
$filesToCopy = @(
    @{ Src = "muhenkan-switch.exe"; Dest = "muhenkan-switch.exe" }
    @{ Src = "muhenkan-switch-core.exe"; Dest = "muhenkan-switch-core.exe" }
    @{ Src = "config.toml";   Dest = "config.toml" }
    @{ Src = "muhenkan.kbd";  Dest = "muhenkan.kbd" }
    @{ Src = "update.ps1";    Dest = "update.ps1" }
    @{ Src = "uninstall.ps1"; Dest = "uninstall.ps1" }
)

foreach ($file in $filesToCopy) {
    $src = Join-Path $ScriptDir $file.Src
    $dest = Join-Path $INSTALL_DIR $file.Dest
    if (Test-Path $src) {
        Copy-Item $src $dest -Force
        Write-Host "[OK] $($file.Src) をコピーしました" -ForegroundColor Green
    } else {
        Write-Host "[SKIP] $($file.Src) が見つかりません" -ForegroundColor Yellow
    }
}

# ── kanata ダウンロード ──
$kanataExe = Join-Path $INSTALL_DIR "kanata_cmd_allowed.exe"
if (Test-Path $kanataExe) {
    Write-Host "[SKIP] kanata_cmd_allowed.exe は既にインストール済みです" -ForegroundColor Yellow
    Write-Host "       再ダウンロードする場合は削除してから再実行してください"
} else {
    Write-Host ""
    Write-Host "kanata $KANATA_VERSION をダウンロードしています..." -ForegroundColor Cyan

    $downloadUrl = "https://github.com/jtroo/kanata/releases/download/$KANATA_VERSION/$KANATA_ASSET"
    $tempZip = Join-Path $env:TEMP "kanata-download.zip"
    $tempExtract = Join-Path $env:TEMP "kanata-extract"

    try {
        # TLS 1.2 を有効化
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

        Invoke-WebRequest -Uri $downloadUrl -OutFile $tempZip -UseBasicParsing
        Write-Host "[OK] ダウンロード完了" -ForegroundColor Green

        # 展開
        if (Test-Path $tempExtract) {
            Remove-Item $tempExtract -Recurse -Force
        }
        Expand-Archive -Path $tempZip -DestinationPath $tempExtract -Force

        # バイナリを探す
        $kanataFile = Get-ChildItem -Path $tempExtract -Recurse -Filter $KANATA_BINARY | Select-Object -First 1
        if ($kanataFile) {
            Copy-Item $kanataFile.FullName $kanataExe -Force
            Write-Host "[OK] kanata_cmd_allowed.exe をインストールしました" -ForegroundColor Green
        } else {
            Write-Host "[ERROR] kanata バイナリが見つかりませんでした: $KANATA_BINARY" -ForegroundColor Red
            Write-Host "        手動でダウンロードしてください: https://github.com/jtroo/kanata/releases" -ForegroundColor Red
        }
    } catch {
        Write-Host "[ERROR] kanata のダウンロードに失敗しました: $_" -ForegroundColor Red
        Write-Host "        手動でダウンロードしてください: https://github.com/jtroo/kanata/releases" -ForegroundColor Red
    } finally {
        # 一時ファイルのクリーンアップ
        if (Test-Path $tempZip) { Remove-Item $tempZip -Force -ErrorAction SilentlyContinue }
        if (Test-Path $tempExtract) { Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

# ── PATH に追加 ──
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$pathEntries = $userPath -split ";" | Where-Object { $_ -ne "" }

if ($pathEntries -contains $INSTALL_DIR) {
    Write-Host "[SKIP] PATH には既に追加済みです" -ForegroundColor Yellow
} else {
    $newPath = ($pathEntries + $INSTALL_DIR) -join ";"
    [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    Write-Host "[OK] ユーザー PATH にインストールディレクトリを追加しました" -ForegroundColor Green
    Write-Host "     ※ 反映にはターミナルの再起動が必要です" -ForegroundColor Yellow
}

# ── スタートアップショートカット（オプション）──
Write-Host ""
$createStartup = Read-Host "スタートアップに muhenkan-switch (GUI) を登録しますか？ (y/N)"
if ($createStartup -eq "y" -or $createStartup -eq "Y") {
    $startupDir = [Environment]::GetFolderPath("Startup")
    $shortcutPath = Join-Path $startupDir "muhenkan-switch.lnk"
    $guiExe = Join-Path $INSTALL_DIR "muhenkan-switch.exe"

    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $guiExe
    $shortcut.WorkingDirectory = $INSTALL_DIR
    $shortcut.Description = "muhenkan-switch (GUI)"
    $shortcut.Save()

    Write-Host "[OK] スタートアップショートカットを作成しました" -ForegroundColor Green
    Write-Host "     $shortcutPath"
}

# ── 完了 ──
Write-Host ""
Write-Host "=== インストール完了 ===" -ForegroundColor Green
Write-Host ""
Write-Host "インストール先: $INSTALL_DIR"
Write-Host ""
Write-Host "使い方:"
Write-Host "  1. ターミナルを再起動してください（PATH の反映）"
Write-Host "  2. muhenkan-switch.exe を起動してください" -ForegroundColor Cyan
Write-Host "     ※ システムトレイに常駐し、kanata を自動管理します"
Write-Host ""
Write-Host "アンインストール: uninstall.ps1 を実行してください"
Write-Host ""
