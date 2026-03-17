[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingArgs = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $here
$targetDirName = if ($env:LGC_TARGET_DIR) { $env:LGC_TARGET_DIR } else { "target" }
$binary = Join-Path $here "$targetDirName\debug\lgc.exe"

function Get-CommanderSourceTimestamp {
    param([string]$WorkspaceRoot)

    $items = @()
    $items += Get-Item (Join-Path $WorkspaceRoot 'Cargo.toml')
    $lockFile = Join-Path $WorkspaceRoot 'Cargo.lock'
    if (Test-Path $lockFile) {
        $items += Get-Item $lockFile
    }
    $items += Get-ChildItem (Join-Path $WorkspaceRoot 'crates') -Recurse -File |
        Where-Object { $_.Extension -in '.rs', '.toml' }

    return ($items | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1).LastWriteTimeUtc
}

function Get-CommanderBinaryVersion {
    param(
        [string]$BinaryPath,
        [string]$ConfigPath
    )

    if (-not (Test-Path $BinaryPath)) {
        return $null
    }

    try {
        $output = & $BinaryPath --config $ConfigPath version 2>$null
        if (-not $output) {
            return $null
        }
        return ($output | Select-Object -First 1)
    }
    catch {
        return $null
    }
}

function Get-ExpectedFrameworkVersion {
    param([string]$WorkspaceRoot)

    $versionFile = Join-Path $WorkspaceRoot 'VERSION'
    if (-not (Test-Path $versionFile)) {
        return $null
    }

    return (Get-Content $versionFile -TotalCount 1).Trim()
}

function Get-LockingCommanderProcesses {
    param([string]$BinaryPath)

    $normalized = [System.IO.Path]::GetFullPath($BinaryPath)
    return @(Get-Process lgc -ErrorAction SilentlyContinue | Where-Object {
            $_.Path -and ([System.IO.Path]::GetFullPath($_.Path) -eq $normalized)
        })
}

function Ensure-CommanderBinary {
    param(
        [string]$WorkspaceRoot,
        [string]$BinaryPath,
        [string]$ConfigPath
    )

    $sourceTimestamp = Get-CommanderSourceTimestamp -WorkspaceRoot $WorkspaceRoot
    $binaryExists = Test-Path $BinaryPath
    $binaryTimestamp = if ($binaryExists) {
        (Get-Item $BinaryPath).LastWriteTimeUtc
    }
    else {
        [datetime]::MinValue
    }

    $expectedVersion = Get-ExpectedFrameworkVersion -WorkspaceRoot $WorkspaceRoot
    $actualVersionLine = Get-CommanderBinaryVersion -BinaryPath $BinaryPath -ConfigPath $ConfigPath
    $versionStale = $false
    if ($expectedVersion -and $actualVersionLine) {
        $versionStale = -not ($actualVersionLine -match [regex]::Escape($expectedVersion))
    }

    $needsBuild = (-not $binaryExists) -or ($sourceTimestamp -gt $binaryTimestamp) -or $versionStale
    if (-not $needsBuild) {
        return
    }

    $locking = @(Get-LockingCommanderProcesses -BinaryPath $BinaryPath)
    if ($locking.Count -gt 0) {
        $pids = ($locking | Select-Object -ExpandProperty Id) -join ', '
        throw "LangGraph-Commander needs a rebuild, but the old panel is still running and locking `"$BinaryPath`" (PID: $pids). Close that commander window and run `commander` again."
    }

    Write-Host "[commander] rebuilding lgc.exe because sources changed..." -ForegroundColor Yellow
    if ($env:LGC_TARGET_DIR) {
        cargo build -p lgc-cli --target-dir $env:LGC_TARGET_DIR
    }
    else {
        cargo build -p lgc-cli
    }
}

Push-Location $here
try {
    $forwarded = @($RemainingArgs)
    if ($forwarded.Count -eq 0) {
        $forwarded = @("tui")
    }

    $configPath = Join-Path $repoRoot "commander.toml"
    Ensure-CommanderBinary -WorkspaceRoot $here -BinaryPath $binary -ConfigPath $configPath
    & $binary --config $configPath @forwarded
}
finally {
    Pop-Location
}
