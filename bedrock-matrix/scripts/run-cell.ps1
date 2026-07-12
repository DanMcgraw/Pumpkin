#Requires -Version 5.1
<#
.SYNOPSIS
    Runs one cell of the Bedrock reproduction matrix.
.DESCRIPTION
    Starts the chosen binary with the chosen runtime, prompts the tester to connect,
    then records the log and shuts down cleanly.

.PARAMETER Binary
    Which binary to run: "original" or "current".

.PARAMETER Runtime
    Which runtime to use: "original-clean" or "runner-copy".

.PARAMETER ResultFile
    Path to the result markdown file to append the log path to.
#>

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("original", "current")]
    [string]$Binary,

    [Parameter(Mandatory = $true)]
    [ValidateSet("original-clean", "runner-copy")]
    [string]$Runtime,

    [Parameter(Mandatory = $false)]
    [string]$ResultFile
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

$BinaryDir = Join-Path $Root "binaries"
$RuntimeDir = Join-Path $Root "runtimes"
$WorkDir = Join-Path $Root "work" "$Binary-$Runtime"
$LogDir = Join-Path $Root "logs"
$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$LogFile = Join-Path $LogDir "$Binary-$Runtime-$Timestamp.log"

New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

$ExeSource = Join-Path $BinaryDir $Binary "pumpkin.exe"
$RuntimeSource = Join-Path $RuntimeDir $Runtime

if (-not (Test-Path $ExeSource)) {
    throw "Binary not found: $ExeSource. Run build.ps1 first."
}
if (-not (Test-Path $RuntimeSource)) {
    throw "Runtime not found: $RuntimeSource. Run setup-runtimes.ps1 first."
}

# Prepare a clean working directory with the runtime + binary.
Write-Host "Preparing work directory: $WorkDir" -ForegroundColor Cyan
if (Test-Path $WorkDir) {
    Remove-Item -Path $WorkDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
Copy-Item -Path $RuntimeSource\* -Destination $WorkDir -Recurse -Force
Copy-Item -Path $ExeSource -Destination (Join-Path $WorkDir "pumpkin.exe") -Force

if (-not (Test-Path (Join-Path $WorkDir "pumpkin.toml"))) {
    Write-Host "No pumpkin.toml found in runtime; the binary will use defaults." -ForegroundColor Yellow
}

Write-Host "`n============================================" -ForegroundColor Green
Write-Host "Cell: binary=$Binary, runtime=$Runtime" -ForegroundColor Green
Write-Host "Working directory: $WorkDir" -ForegroundColor Green
Write-Host "Server log: $LogFile" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green

# Use cmd to redirect both stdout and stderr to the same log file.
$cmdArgs = "/c `"`"$($WorkDir)\pumpkin.exe`" > `"$LogFile`" 2>&1`""
$proc = Start-Process -FilePath "cmd.exe" -ArgumentList $cmdArgs -WorkingDirectory $WorkDir -PassThru

Write-Host "`nServer started (PID $($proc.Id))." -ForegroundColor Cyan
Write-Host "1. Connect the same Bedrock client you used for the other cells." -ForegroundColor Yellow
Write-Host "2. Observe whether it reaches the world and stays connected." -ForegroundColor Yellow
Write-Host "3. Press ENTER here when you are done to stop the server." -ForegroundColor Yellow
Read-Host

Write-Host "Stopping server ..." -ForegroundColor Cyan
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
$proc.WaitForExit(5000) | Out-Null

Write-Host "Server stopped. Log saved to: $LogFile" -ForegroundColor Green

if ($ResultFile) {
    $ResultDir = Split-Path -Parent $ResultFile
    New-Item -ItemType Directory -Force -Path $ResultDir | Out-Null
    $entry = @"

## Run $Timestamp
- Binary: $Binary
- Runtime: $Runtime
- Log: $LogFile
- Outcome: (fill in)
"@
    Add-Content -Path $ResultFile -Value $entry
    Write-Host "Result entry appended to: $ResultFile" -ForegroundColor Green
}
