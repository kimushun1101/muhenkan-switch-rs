# dev.ps1 — ローカル開発用スクリプト
# 使い方:
#   .\dev.ps1          muhenkan-switch をビルドしてルートにコピー
#   .\dev.ps1 run      ビルド後、kanata も起動
#   .\dev.ps1 release  --release ビルド

param(
    [Parameter(Position = 0)]
    [ValidateSet("build", "run", "release")]
    [string]$Action = "build"
)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot

# ── ビルド ──
$buildArgs = @()
if ($Action -eq "release") {
    $buildArgs += "--release"
    $targetDir = "release"
} else {
    $targetDir = "debug"
}

Write-Host "[dev] Building muhenkan-switch ($targetDir)..." -ForegroundColor Cyan
Push-Location "$Root\muhenkan-switch"
try {
    cargo build @buildArgs
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
} finally {
    Pop-Location
}

# ── コピー ──
$src = "$Root\muhenkan-switch\target\$targetDir\muhenkan-switch.exe"
$dst = "$Root\muhenkan-switch.exe"
Copy-Item $src $dst -Force
Write-Host "[dev] Copied -> $dst" -ForegroundColor Green

# ── kanata 起動 ──
if ($Action -eq "run") {
    $kanata = "$Root\kanata_cmd_allowed.exe"
    $kbd = "$Root\kanata\muhenkan.kbd"
    if (-not (Test-Path $kanata)) {
        Write-Host "[dev] kanata_cmd_allowed.exe が見つかりません。" -ForegroundColor Red
        Write-Host "[dev] インストールスクリプトを実行するか、手動で配置してください。" -ForegroundColor Red
        exit 1
    }
    Write-Host "[dev] Starting kanata..." -ForegroundColor Cyan
    Write-Host "[dev] Ctrl+C で終了" -ForegroundColor DarkGray
    & $kanata --cfg $kbd
}
