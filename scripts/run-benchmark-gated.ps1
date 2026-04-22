[CmdletBinding(PositionalBinding = $false)]
param(
    [ValidateSet("smoke", "development", "authoritative")]
    [string]$Mode = "development",

    [int]$Workers,

    [switch]$WaitForQuiet,

    [int]$MaxWaitMinutes = 180,

    [int]$PollIntervalSeconds = 15,

    [switch]$SkipEnvVerification,

    [switch]$SkipMutex,

    [switch]$PrintPlan,

    [Parameter(Mandatory = $true)]
    [string]$BenchmarkCommandLine
)

$ErrorActionPreference = "Stop"

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "=== $Title ==="
}

function Get-ModePolicy {
    param([string]$SelectedMode)

    switch ($SelectedMode) {
        "smoke" {
            return [pscustomobject]@{
                DefaultWorkers = 4
                MaxCpuLoadPercent = 85
                MinFreeRamGiB = 6
                MinFreeDiskGiB = 5
                PriorityClass = "Normal"
                WaitForQuietByDefault = $false
                SerializeHeavy = $false
            }
        }
        "development" {
            return [pscustomobject]@{
                DefaultWorkers = 8
                MaxCpuLoadPercent = 60
                MinFreeRamGiB = 8
                MinFreeDiskGiB = 10
                PriorityClass = "AboveNormal"
                WaitForQuietByDefault = $false
                SerializeHeavy = $true
            }
        }
        "authoritative" {
            return [pscustomobject]@{
                DefaultWorkers = 8
                MaxCpuLoadPercent = 20
                MinFreeRamGiB = 12
                MinFreeDiskGiB = 20
                PriorityClass = "High"
                WaitForQuietByDefault = $true
                SerializeHeavy = $true
            }
        }
        default {
            throw "Unsupported mode: $SelectedMode"
        }
    }
}

function Test-HeavyBenchmarkCommand {
    param([string]$CommandLine)

    $patterns = @(
        'traceboost-app(?:\.exe)?\s+benchmark-trace-local-processing',
        'traceboost-app(?:\.exe)?\s+benchmark-trace-local-batch-processing',
        'benchmark_desktop_preview_session_',
        'benchmark_processing_cache_',
        'cargo\s+bench',
        'criterion'
    )

    foreach ($pattern in $patterns) {
        if ($CommandLine -match $pattern) {
            return $true
        }
    }

    return $false
}

function Get-MachineSnapshot {
    $cpu = Get-CimInstance Win32_Processor | Measure-Object -Property NumberOfCores, NumberOfLogicalProcessors -Sum
    $os = Get-CimInstance Win32_OperatingSystem
    $cpuLoad = (Get-Counter '\Processor(_Total)\% Processor Time' -SampleInterval 1 -MaxSamples 1).CounterSamples[0].CookedValue

    [pscustomobject]@{
        PhysicalCores = [int]$cpu.Sum[0]
        LogicalProcessors = [int]$cpu.Sum[1]
        CpuLoadPercent = [math]::Round($cpuLoad, 1)
        FreeRamGiB = [math]::Round(($os.FreePhysicalMemory * 1KB) / 1GB, 2)
        TotalRamGiB = [math]::Round(($os.TotalVisibleMemorySize * 1KB) / 1GB, 2)
    }
}

function Get-RelevantDriveInfo {
    param([string]$CommandLine)

    $roots = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    $roots.Add("C:") | Out-Null

    foreach ($match in [System.Text.RegularExpressions.Regex]::Matches($CommandLine, '[A-Za-z]:\\')) {
        $roots.Add($match.Value.Substring(0, 2)) | Out-Null
    }

    $result = New-Object System.Collections.Generic.List[object]
    foreach ($root in $roots) {
        $drive = Get-CimInstance Win32_LogicalDisk -Filter ("DeviceID='{0}'" -f $root)
        if ($null -ne $drive) {
            $result.Add([pscustomobject]@{
                DeviceId = $root
                FreeGiB = [math]::Round($drive.FreeSpace / 1GB, 2)
                SizeGiB = [math]::Round($drive.Size / 1GB, 2)
            })
        }
    }

    return $result
}

function Get-ContendingProcesses {
    param([int]$LogicalProcessors)

    try {
        $samples = (Get-Counter '\Process(*)\% Processor Time' -SampleInterval 1 -MaxSamples 1).CounterSamples |
            Where-Object {
                $_.Status -eq 0 -and $_.InstanceName -notmatch '^(idle|_total)$'
            }
    } catch {
        return @()
    }

    $byName = $samples |
        Group-Object InstanceName |
        ForEach-Object {
            $cpu = (($_.Group | Measure-Object -Property CookedValue -Sum).Sum / [math]::Max($LogicalProcessors, 1))
            [pscustomobject]@{
                ProcessName = $_.Name
                CpuPercent = [math]::Round($cpu, 1)
            }
        } |
        Sort-Object CpuPercent -Descending

    return $byName | Where-Object {
        $_.CpuPercent -ge 5.0 -and $_.ProcessName -notin @("system", "dwm", "taskmgr")
    }
}

function Resolve-LaunchSpec {
    param([string]$CommandLine)

    if ([string]::IsNullOrWhiteSpace($CommandLine)) {
        throw "A benchmark command is required."
    }

    return [pscustomobject]@{
        FilePath = "cmd.exe"
        Arguments = @("/d", "/c", $CommandLine)
    }
}

function Normalize-BenchmarkCommandLine {
    param(
        [string]$CommandLine,
        [string]$RepoRoot
    )

    $trimmed = $CommandLine.Trim()
    $cargoPattern = '^(?:\.\\)?cargo(?:\.exe)?\b'
    $usesWindowsMsvcCargo = $false
    $normalized = $trimmed

    if ($env:OS -eq "Windows_NT" -and $trimmed -match $cargoPattern) {
        $wrapperPath = Join-Path $RepoRoot "scripts\windows-msvc-cargo.cmd"
        $quotedWrapper = '"' + $wrapperPath + '"'
        $normalized = [System.Text.RegularExpressions.Regex]::Replace(
            $trimmed,
            $cargoPattern,
            [System.Text.RegularExpressions.MatchEvaluator]{ param($m) $quotedWrapper },
            1
        )
        $usesWindowsMsvcCargo = $true
    }

    [pscustomobject]@{
        CommandLine = $normalized
        UsesWindowsMsvcCargo = $usesWindowsMsvcCargo
    }
}

function Test-QuietEnough {
    param(
        [pscustomobject]$Snapshot,
        [System.Collections.Generic.List[object]]$Drives,
        [System.Object[]]$Contenders,
        [pscustomobject]$Policy
    )

    if ($Snapshot.CpuLoadPercent -gt $Policy.MaxCpuLoadPercent) {
        return $false
    }

    if ($Snapshot.FreeRamGiB -lt $Policy.MinFreeRamGiB) {
        return $false
    }

    foreach ($drive in $Drives) {
        if ($drive.FreeGiB -lt $Policy.MinFreeDiskGiB) {
            return $false
        }
    }

    if ($Contenders.Count -gt 0) {
        return $false
    }

    return $true
}

function Write-LaunchPlan {
    param(
        [string]$SelectedMode,
        [bool]$HeavyBenchmark,
        [int]$WorkerCount,
        [bool]$RequiresQuiet,
        [bool]$RequiresMutex,
        [bool]$UsesWindowsMsvcCargo,
        [pscustomobject]$Snapshot,
        [System.Collections.Generic.List[object]]$Drives,
        [System.Object[]]$Contenders,
        [string]$CommandLine
    )

    Write-Section "Benchmark Launch Plan"
    Write-Host "mode=$SelectedMode"
    Write-Host "heavy_benchmark=$HeavyBenchmark"
    Write-Host "workers=$WorkerCount"
    Write-Host "requires_quiet_machine=$RequiresQuiet"
    Write-Host "requires_heavy_lane=$RequiresMutex"
    Write-Host "uses_windows_msvc_cargo=$UsesWindowsMsvcCargo"
    Write-Host "command=$CommandLine"

    Write-Section "Machine Snapshot"
    Write-Host "physical_cores=$($Snapshot.PhysicalCores)"
    Write-Host "logical_processors=$($Snapshot.LogicalProcessors)"
    Write-Host "cpu_load_percent=$($Snapshot.CpuLoadPercent)"
    Write-Host "free_ram_gib=$($Snapshot.FreeRamGiB)"
    Write-Host "total_ram_gib=$($Snapshot.TotalRamGiB)"

    Write-Section "Disk Snapshot"
    foreach ($drive in $Drives) {
        Write-Host "$($drive.DeviceId) free_gib=$($drive.FreeGiB) size_gib=$($drive.SizeGiB)"
    }

    Write-Section "Contending Processes"
    if ($Contenders.Count -eq 0) {
        Write-Host "none_above_threshold=true"
    } else {
        foreach ($contender in $Contenders) {
            Write-Host "$($contender.ProcessName) cpu_percent=$($contender.CpuPercent)"
        }
    }
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$rawBenchmarkCommandLine = $BenchmarkCommandLine
$normalizedCommand = Normalize-BenchmarkCommandLine -CommandLine $BenchmarkCommandLine -RepoRoot $repoRoot
$BenchmarkCommandLine = $normalizedCommand.CommandLine
$policy = Get-ModePolicy -SelectedMode $Mode
$heavyBenchmark = (Test-HeavyBenchmarkCommand -CommandLine $rawBenchmarkCommandLine) -or
    (Test-HeavyBenchmarkCommand -CommandLine $BenchmarkCommandLine)
$requiresMutex = $heavyBenchmark -and $policy.SerializeHeavy -and (-not $SkipMutex)
$requiresQuiet = $heavyBenchmark -and ($WaitForQuiet -or $policy.WaitForQuietByDefault)

$workerCount = $null
if ($PSBoundParameters.ContainsKey("Workers")) {
    $workerCount = $Workers
} elseif (-not [string]::IsNullOrWhiteSpace($env:OPHIOLITE_BENCHMARK_WORKERS)) {
    $parsedWorkers = 0
    if ([int]::TryParse($env:OPHIOLITE_BENCHMARK_WORKERS, [ref]$parsedWorkers)) {
        $workerCount = $parsedWorkers
    }
}

if ($null -eq $workerCount -or $workerCount -le 0) {
    $workerCount = $policy.DefaultWorkers
}

$snapshot = Get-MachineSnapshot
$drives = Get-RelevantDriveInfo -CommandLine $BenchmarkCommandLine
$contenders = Get-ContendingProcesses -LogicalProcessors $snapshot.LogicalProcessors

Write-LaunchPlan `
    -SelectedMode $Mode `
    -HeavyBenchmark $heavyBenchmark `
    -WorkerCount $workerCount `
    -RequiresQuiet $requiresQuiet `
    -RequiresMutex $requiresMutex `
    -UsesWindowsMsvcCargo $normalizedCommand.UsesWindowsMsvcCargo `
    -Snapshot $snapshot `
    -Drives $drives `
    -Contenders $contenders `
    -CommandLine $BenchmarkCommandLine

if ($PrintPlan) {
    exit 0
}

if (-not $SkipEnvVerification -and $Mode -eq "authoritative") {
    Write-Section "Environment Verification"
    & (Join-Path $PSScriptRoot "verify-windows-benchmark-env.ps1")
    if ($LASTEXITCODE -ne 0) {
        throw "Benchmark environment verification failed."
    }
}

$mutex = $null
$mutexAcquired = $false
$exitCode = 0

try {
    if ($requiresMutex) {
        Write-Section "Heavy Lane"
        $mutex = New-Object System.Threading.Mutex($false, "Global\OphioliteBenchmarkHeavyLane")
        $waitTimeout = [TimeSpan]::FromMinutes($MaxWaitMinutes)

        try {
            $mutexAcquired = $mutex.WaitOne($waitTimeout)
        } catch [System.Threading.AbandonedMutexException] {
            $mutexAcquired = $true
            Write-Host "recovered_abandoned_heavy_lane=true"
        }

        if (-not $mutexAcquired) {
            throw "Timed out waiting for the heavy benchmark lane."
        }

        Write-Host "heavy_lane_acquired=true"
    }

    if ($requiresQuiet) {
        Write-Section "Quiet-Machine Gate"
        $deadline = (Get-Date).AddMinutes($MaxWaitMinutes)

        while ($true) {
            $snapshot = Get-MachineSnapshot
            $drives = Get-RelevantDriveInfo -CommandLine $BenchmarkCommandLine
            $contenders = Get-ContendingProcesses -LogicalProcessors $snapshot.LogicalProcessors

            if (Test-QuietEnough -Snapshot $snapshot -Drives $drives -Contenders $contenders -Policy $policy) {
                Write-Host "quiet_machine_ready=true"
                break
            }

            if ((Get-Date) -ge $deadline) {
                throw "Timed out waiting for a quiet machine state for authoritative benchmarking."
            }

            Write-Host "quiet_machine_ready=false"
            Write-Host "waiting_seconds=$PollIntervalSeconds"
            Start-Sleep -Seconds $PollIntervalSeconds
        }
    } elseif ($heavyBenchmark -and $contenders.Count -gt 0) {
        Write-Section "Contention Warning"
        Write-Host "Benchmark will run in a contended state. Treat the result as exploratory."
    }

    $env:OPHIOLITE_BENCHMARK_WORKERS = [string]$workerCount
    $env:RAYON_NUM_THREADS = [string]$workerCount

    $launchSpec = Resolve-LaunchSpec -CommandLine $BenchmarkCommandLine

    Write-Section "Launching Benchmark"
    Write-Host "file_path=$($launchSpec.FilePath)"
    Write-Host "arguments=$([string]::Join(' ', $launchSpec.Arguments))"
    Write-Host "priority_class=$($policy.PriorityClass)"
    Write-Host "OPHIOLITE_BENCHMARK_WORKERS=$env:OPHIOLITE_BENCHMARK_WORKERS"
    Write-Host "RAYON_NUM_THREADS=$env:RAYON_NUM_THREADS"

    $process = Start-Process -FilePath $launchSpec.FilePath `
        -ArgumentList $launchSpec.Arguments `
        -WorkingDirectory $repoRoot `
        -NoNewWindow `
        -PassThru

    try {
        $process.PriorityClass = $policy.PriorityClass
    } catch {
        Write-Host "priority_update_failed=$($_.Exception.Message)"
    }

    $null = $process.WaitForExit()
    $exitCode = $process.ExitCode
} finally {
    if ($mutexAcquired -and $null -ne $mutex) {
        $mutex.ReleaseMutex()
    }

    if ($null -ne $mutex) {
        $mutex.Dispose()
    }
}

exit $exitCode
