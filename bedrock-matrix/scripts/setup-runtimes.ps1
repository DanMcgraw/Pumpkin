#Requires -Version 5.1
<#
.SYNOPSIS
    Copies runtime directories into the matrix without modifying the originals.
.DESCRIPTION
    Produces:
      - runtimes/original-clean/   (copy of ../Original-Pumpkin runtime)
      - runtimes/runner-copy/      (copy of ../PumpkinRunner runtime)
#>

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ReposRoot = Split-Path -Parent $Root

$RuntimeDir = Join-Path $Root "runtimes"
$OriginalRepo = Join-Path $ReposRoot "Original-Pumpkin"
$RunnerDir = Join-Path $ReposRoot "PumpkinRunner"

function Copy-Runtime {
    param(
        [string]$SourceDir,
        [string]$DestDir,
        [string]$Label
    )

    Write-Host "Copying $Label runtime ..." -ForegroundColor Cyan

    if (-not (Test-Path $SourceDir)) {
        throw "Source runtime not found: $SourceDir"
    }

    if (Test-Path $DestDir) {
        Write-Host "  Destination already exists: $DestDir" -ForegroundColor Yellow
        Write-Host "  Remove it first if you want a fresh copy." -ForegroundColor Yellow
        return
    }

    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null

    # Copy only the runtime-relevant subdirectories/files.
    $Items = @(
        "data",
        "logs",
        "plugins",
        "world",
        "pumpkin.toml"
    )

    foreach ($item in $Items) {
        $src = Join-Path $SourceDir $item
        if (Test-Path $src) {
            Copy-Item -Path $src -Destination $DestDir -Recurse -Force
            Write-Host "  copied $item" -ForegroundColor Gray
        }
        else {
            Write-Host "  skipped missing $item" -ForegroundColor DarkGray
        }
    }

    Write-Host "$Label runtime copied -> $DestDir" -ForegroundColor Green
}

Copy-Runtime -SourceDir $OriginalRepo -DestDir (Join-Path $RuntimeDir "original-clean") -Label "Original-Pumpkin clean"
Copy-Runtime -SourceDir $RunnerDir -DestDir (Join-Path $RuntimeDir "runner-copy") -Label "PumpkinRunner"

Write-Host "`nRuntime copies are ready in $RuntimeDir" -ForegroundColor Green
