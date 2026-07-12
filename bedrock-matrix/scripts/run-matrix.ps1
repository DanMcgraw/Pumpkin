#Requires -Version 5.1
<#
.SYNOPSIS
    Runs all four cells of the Bedrock reproduction matrix sequentially.
.DESCRIPTION
    Prompts the tester before each cell so the same Bedrock client can be used.
    Results are appended to the markdown files under results/.
#>

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ScriptDir = Join-Path $Root "scripts"
$ResultDir = Join-Path $Root "results"

New-Item -ItemType Directory -Force -Path $ResultDir | Out-Null

$cells = @(
    @{ Binary = "original"; Runtime = "original-clean"; Result = "original-original.md"; Label = "A - baseline" },
    @{ Binary = "original"; Runtime = "runner-copy"; Result = "original-runner.md"; Label = "B - data test" },
    @{ Binary = "current"; Runtime = "original-clean"; Result = "current-original.md"; Label = "C - binary test" },
    @{ Binary = "current"; Runtime = "runner-copy"; Result = "current-runner.md"; Label = "D - failing case" }
)

Write-Host "`nBedrock Reproduction Matrix" -ForegroundColor Green
Write-Host "===========================" -ForegroundColor Green
Write-Host "Use the SAME Bedrock client for all four cells.`n" -ForegroundColor Yellow

foreach ($cell in $cells) {
    Write-Host "`n----------------------------------------" -ForegroundColor Cyan
    Write-Host "Next cell: $($cell.Label)" -ForegroundColor Cyan
    Write-Host "  Binary : $($cell.Binary)" -ForegroundColor Cyan
    Write-Host "  Runtime: $($cell.Runtime)" -ForegroundColor Cyan
    Write-Host "----------------------------------------" -ForegroundColor Cyan
    Read-Host "Press ENTER to start this cell (make sure the previous server is stopped)"

    $resultPath = Join-Path $ResultDir $cell.Result
    $runCell = Join-Path $ScriptDir "run-cell.ps1"
    & $runCell -Binary $cell.Binary -Runtime $cell.Runtime -ResultFile $resultPath

    Write-Host "`nCell $($cell.Label) complete. Result file: $resultPath" -ForegroundColor Green
}

Write-Host "`nAll cells complete. Fill in results/*.md using results/template.md." -ForegroundColor Green
