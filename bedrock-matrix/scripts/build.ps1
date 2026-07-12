#Requires -Version 5.1
<#
.SYNOPSIS
    Builds the Original-Pumpkin and current Pumpkin binaries for the matrix.
.DESCRIPTION
    Produces:
      - binaries/original/pumpkin.exe
      - binaries/current/pumpkin.exe
    Uses debug builds by default for fast iteration.
#>

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ProjectRoot = Split-Path -Parent $Root
$ReposRoot = Split-Path -Parent $ProjectRoot

$BinaryDir = Join-Path $Root "binaries"
$OriginalRepo = Join-Path $ReposRoot "Original-Pumpkin"
$CurrentRepo = Join-Path $ReposRoot "Pumpkin"

function Build-Project {
    param(
        [string]$RepoPath,
        [string]$OutputDir,
        [string]$Label
    )

    Write-Host "Building $Label from $RepoPath ..." -ForegroundColor Cyan

    if (-not (Test-Path $RepoPath)) {
        throw "Repository not found: $RepoPath"
    }

    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

    Push-Location $RepoPath
    try {
        cargo build -p pumpkin --bin pumpkin
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for $Label"
        }
    }
    finally {
        Pop-Location
    }

    # Respect per-project target-dir overrides (e.g. Pumpkin's .cargo/config.toml
    # sets target-dir = "../PumpkinRunner/target").
    $metadata = & cargo metadata --format-version 1 --no-deps --manifest-path (Join-Path $RepoPath "Cargo.toml") | ConvertFrom-Json
    $TargetDir = [System.IO.Path]::GetFullPath($metadata.target_directory)
    $SourceExe = Join-Path $TargetDir "debug\pumpkin.exe"
    if (-not (Test-Path $SourceExe)) {
        throw "Expected build output not found: $SourceExe"
    }

    Copy-Item -Path $SourceExe -Destination $OutputDir -Force
    Write-Host "$Label built -> $OutputDir\pumpkin.exe" -ForegroundColor Green
}

Build-Project -RepoPath $OriginalRepo -OutputDir (Join-Path $BinaryDir "original") -Label "Original-Pumpkin"
Build-Project -RepoPath $CurrentRepo -OutputDir (Join-Path $BinaryDir "current") -Label "current Pumpkin"

Write-Host "`nBoth binaries are ready in $BinaryDir" -ForegroundColor Green
