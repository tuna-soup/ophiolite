param(
    [switch]$RunCargoCheck,
    [string]$RuntimePackage = "ophiolite-seismic-runtime"
)

$ErrorActionPreference = "Stop"

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "=== $Title ==="
}

function Test-PathEntry {
    param(
        [string[]]$Entries,
        [string]$Pattern
    )

    foreach ($entry in $Entries) {
        if ($entry -match $Pattern) {
            return $true
        }
    }

    return $false
}

function Resolve-SqliteLibrary {
    param([string]$LibDir)

    $candidates = @(
        "sqlite3.lib",
        "libsqlite3.lib",
        "libsqlite3.a"
    )

    foreach ($candidate in $candidates) {
        $path = Join-Path $LibDir $candidate
        if (Test-Path $path) {
            return $path
        }
    }

    return $null
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$pathEntries = $env:PATH -split ';' | Where-Object { $_ }
$warnings = [System.Collections.Generic.List[string]]::new()
$errors = [System.Collections.Generic.List[string]]::new()

Write-Section "Machine"
$computer = Get-CimInstance Win32_ComputerSystem
$os = Get-CimInstance Win32_OperatingSystem
$cpuCounter = Get-Counter '\Processor(_Total)\% Processor Time'
$cpuLoad = [math]::Round($cpuCounter.CounterSamples[0].CookedValue, 1)
$freeRamGiB = [math]::Round(($os.FreePhysicalMemory * 1KB) / 1GB, 1)
$totalRamGiB = [math]::Round(($os.TotalVisibleMemorySize * 1KB) / 1GB, 1)
Write-Host "logical_processors=$($computer.NumberOfLogicalProcessors)"
Write-Host "cpu_load_percent=$cpuLoad"
Write-Host "free_ram_gib=$freeRamGiB / total_ram_gib=$totalRamGiB"

if ($cpuLoad -gt 20) {
    $warnings.Add("CPU load is $cpuLoad%. Use a quieter machine state for authoritative benchmarks.")
}

if ($freeRamGiB -lt 12) {
    $warnings.Add("Free RAM is only ${freeRamGiB} GiB. Large F3 benchmark runs may page or lose stability.")
}

Write-Section "Toolchain"
$rustc = & rustc -Vv
$cargo = & cargo -V
Write-Host ($rustc | Select-String '^host:' | ForEach-Object { $_.ToString().Trim() })
Write-Host $cargo

if (-not ((& rustc -Vv) -match 'host: x86_64-pc-windows-msvc')) {
    $errors.Add("Rust host toolchain is not x86_64-pc-windows-msvc.")
}

$cl = Get-Command cl.exe -ErrorAction SilentlyContinue
if ($null -eq $cl) {
    $warnings.Add("cl.exe is not currently on PATH. Launch from Developer PowerShell / VsDevCmd or use scripts/windows-msvc-cargo.cmd.")
} else {
    Write-Host "cl=$($cl.Source)"
}

Write-Section "PATH Hygiene"
$pathWarnings = @(
    @{ Label = "msys64"; Pattern = 'C:\\msys64\\ucrt64\\bin' }
    @{ Label = "strawberry-c"; Pattern = 'C:\\Strawberry\\c\\bin' }
    @{ Label = "strawberry-perl"; Pattern = 'C:\\Strawberry\\perl' }
    @{ Label = "mingw-qt"; Pattern = 'C:\\Qt\\6\.8\.1\\mingw_64\\bin' }
)

foreach ($entry in $pathWarnings) {
    $present = Test-PathEntry -Entries $pathEntries -Pattern $entry.Pattern
    Write-Host "$($entry.Label)=$present"
    if ($present) {
        $warnings.Add("PATH contains $($entry.Label), which can contaminate MSVC CMake builds.")
    }
}

Write-Section "SQLite"
$sqliteInclude = $env:OPHIOLITE_SQLITE_INCLUDE
$sqliteLibDir = $env:OPHIOLITE_SQLITE_LIB_DIR
$sqliteBinDir = $env:OPHIOLITE_SQLITE_BIN_DIR

Write-Host "OPHIOLITE_SQLITE_INCLUDE=$sqliteInclude"
Write-Host "OPHIOLITE_SQLITE_LIB_DIR=$sqliteLibDir"
Write-Host "OPHIOLITE_SQLITE_BIN_DIR=$sqliteBinDir"

if ([string]::IsNullOrWhiteSpace($sqliteInclude)) {
    $errors.Add("OPHIOLITE_SQLITE_INCLUDE is not set.")
} elseif (-not (Test-Path (Join-Path $sqliteInclude "sqlite3.h"))) {
    $errors.Add("sqlite3.h not found under OPHIOLITE_SQLITE_INCLUDE.")
}

if ([string]::IsNullOrWhiteSpace($sqliteLibDir)) {
    $errors.Add("OPHIOLITE_SQLITE_LIB_DIR is not set.")
} else {
    $sqliteLibrary = Resolve-SqliteLibrary -LibDir $sqliteLibDir
    if ($null -eq $sqliteLibrary) {
        $errors.Add("No sqlite3.lib / libsqlite3.lib / libsqlite3.a found under OPHIOLITE_SQLITE_LIB_DIR.")
    } else {
        Write-Host "sqlite_library=$sqliteLibrary"
    }
}

if (-not [string]::IsNullOrWhiteSpace($sqliteBinDir) -and -not (Test-Path (Join-Path $sqliteBinDir "sqlite3.exe"))) {
    $errors.Add("sqlite3.exe not found under OPHIOLITE_SQLITE_BIN_DIR.")
}

Write-Section "Benchmark Binary"
$traceboostBinary = Join-Path $repoRoot "target\debug\traceboost-app.exe"
if (Test-Path $traceboostBinary) {
    $binary = Get-Item $traceboostBinary
    Write-Host "traceboost-app=$($binary.FullName)"
    Write-Host "last_write_time=$($binary.LastWriteTime)"
    Write-Host "size_bytes=$($binary.Length)"
} else {
    $warnings.Add("target\\debug\\traceboost-app.exe is missing.")
}

if ($RunCargoCheck) {
    Write-Section "Cargo Check"
    Push-Location $repoRoot
    try {
        & "$repoRoot\scripts\windows-msvc-cargo.cmd" check -p $RuntimePackage --lib --tests
        if ($LASTEXITCODE -ne 0) {
            $errors.Add("windows-msvc-cargo.cmd cargo check failed for package $RuntimePackage.")
        }
    } finally {
        Pop-Location
    }
}

Write-Section "Summary"
if ($warnings.Count -gt 0) {
    Write-Host "warnings:"
    foreach ($warning in $warnings) {
        Write-Host " - $warning"
    }
}

if ($errors.Count -gt 0) {
    Write-Host "errors:"
    foreach ($entry in $errors) {
        Write-Host " - $entry"
    }
    exit 1
}

Write-Host "environment looks usable for MSVC build verification."
if ($warnings.Count -gt 0) {
    Write-Host "benchmarking should still wait until the warnings are addressed."
}
