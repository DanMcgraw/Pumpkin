#Requires -Version 5.1
<#
.SYNOPSIS
    Compares the original-clean and runner-copy runtimes to find likely crash triggers.
.DESCRIPTION
    Produces a side-by-side report of config, world, player data, plugins, and entity-related files.
    Does not parse NBT deeply; use an NBT editor for detailed entity inspection.
#>

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ProjectRoot = Split-Path -Parent $Root
$ReposRoot = Split-Path -Parent $ProjectRoot

$OriginalRuntime = Join-Path $Root "runtimes\original-clean"
$RunnerRuntime = Join-Path $Root "runtimes\runner-copy"

function Test-RuntimeDir {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        throw "Runtime not found: $Path. Run setup-runtimes.ps1 first."
    }
}

Test-RuntimeDir -Path $OriginalRuntime
Test-RuntimeDir -Path $RunnerRuntime

$Report = [System.Collections.Generic.List[string]]::new()
$Report.Add('# Runtime Comparison Report')
$Report.Add('')
$Report.Add("Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')")
$Report.Add('')
$Report.Add('| Path | Original clean | Runner copy |')
$Report.Add('|------|----------------|-------------|')

function Add-Row {
    param([string]$Label, [string]$Original, [string]$Runner)
    $Report.Add("| $Label | $Original | $Runner |")
}

function Get-ItemCount {
    param([string]$Path)
    if (Test-Path $Path) {
        return (Get-ChildItem -Path $Path -Recurse -File -ErrorAction SilentlyContinue).Count
    }
    return 0
}

function Get-DirSize {
    param([string]$Path)
    if (Test-Path $Path) {
        $bytes = (Get-ChildItem -Path $Path -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
        return "$([math]::Round($bytes / 1MB, 2)) MB"
    }
    return "0 MB"
}

function Get-RegionCount {
    param([string]$Path)
    if (Test-Path $Path) {
        return (Get-ChildItem -Path $Path -Filter "*.pump" -Recurse -ErrorAction SilentlyContinue).Count
    }
    return 0
}

function Get-PluginNames {
    param([string]$Path)
    if (-not (Test-Path $Path)) { return "none" }
    $dirs = Get-ChildItem -Path $Path -Directory -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name
    if ($dirs) { return ($dirs -join ", ") }
    return "none"
}

# High-level counts
Add-Row -Label "World size" -Original (Get-DirSize -Path "$OriginalRuntime\world") -Runner (Get-DirSize -Path "$RunnerRuntime\world")
Add-Row -Label "World files" -Original (Get-ItemCount -Path "$OriginalRuntime\world") -Runner (Get-ItemCount -Path "$RunnerRuntime\world")
Add-Row -Label "Region files (.pump)" -Original (Get-RegionCount -Path "$OriginalRuntime\world") -Runner (Get-RegionCount -Path "$RunnerRuntime\world")
Add-Row -Label "Player data files" -Original (Get-ItemCount -Path "$OriginalRuntime\data") -Runner (Get-ItemCount -Path "$RunnerRuntime\data")
Add-Row -Label "Plugins" -Original (Get-ItemCount -Path "$OriginalRuntime\plugins") -Runner (Get-ItemCount -Path "$RunnerRuntime\plugins")
Add-Row -Label "Plugin names" -Original (Get-PluginNames -Path "$OriginalRuntime\plugins") -Runner (Get-PluginNames -Path "$RunnerRuntime\plugins")

$Report.Add('')
$Report.Add('## pumpkin.toml differences')
$Report.Add('')

$OriginalConfig = Join-Path $OriginalRuntime "pumpkin.toml"
$RunnerConfig = Join-Path $RunnerRuntime "pumpkin.toml"

if ((Test-Path $OriginalConfig) -and (Test-Path $RunnerConfig)) {
    $diff = Compare-Object -ReferenceObject (Get-Content $OriginalConfig) -DifferenceObject (Get-Content $RunnerConfig)
    if ($diff) {
        $Report.Add('```diff')
        foreach ($line in $diff) {
            $prefix = switch ($line.SideIndicator) {
                "=>" { "+ " }
                "<=" { "- " }
                default { "  " }
            }
            $Report.Add("$prefix$($line.InputObject)")
        }
        $Report.Add('```')
    }
    else {
        $Report.Add('No differences found.')
    }
}
else {
    $Report.Add('One or both pumpkin.toml files are missing.')
}

$Report.Add('')
$Report.Add('## Region file counts by dimension')
$Report.Add('')

function Get-RegionFilesByFolder {
    param([string]$WorldPath)
    $result = @{}
    if (-not (Test-Path $WorldPath)) { return $result }
    foreach ($dir in Get-ChildItem -Path $WorldPath -Directory -Recurse -ErrorAction SilentlyContinue) {
        $mcaCount = (Get-ChildItem -Path $dir.FullName -Filter "*.pump" -ErrorAction SilentlyContinue).Count
        if ($mcaCount -gt 0) {
            $relative = $dir.FullName.Substring($WorldPath.Length).TrimStart('\', '/')
            $result[$relative] = $mcaCount
        }
    }
    return $result
}

$OriginalRegions = Get-RegionFilesByFolder -WorldPath "$OriginalRuntime\world"
$RunnerRegions = Get-RegionFilesByFolder -WorldPath "$RunnerRuntime\world"

$allKeys = ($OriginalRegions.Keys + $RunnerRegions.Keys) | Sort-Object -Unique
$Report.Add('| Dimension/Folder | Original | Runner |')
$Report.Add('|------------------|----------|--------|')
foreach ($key in $allKeys) {
    $orig = if ($OriginalRegions.ContainsKey($key)) { $OriginalRegions[$key] } else { 0 }
    $run = if ($RunnerRegions.ContainsKey($key)) { $RunnerRegions[$key] } else { 0 }
    $Report.Add("| $key | $orig | $run |")
}

$Report.Add('')
$Report.Add('## Suggested next steps')
$Report.Add('')
$Report.Add('1. If world size or region counts differ significantly, the Runner world has generated/explored far more chunks.')
$Report.Add('2. If pumpkin.toml differs, note view-distance, simulation-distance, and any Bedrock-specific settings.')
$Report.Add('3. If plugins differ, test without plugins first.')
$Report.Add('4. Use an NBT editor to inspect entities near spawn in the Runner world''s region files.')
$Report.Add('5. Look for saved player data that places the join position inside or near unusual entities/blocks.')

$ReportPath = Join-Path $Root "runtime-comparison.md"
$Report | Set-Content -Path $ReportPath
Write-Host "Comparison report written to: $ReportPath" -ForegroundColor Green
