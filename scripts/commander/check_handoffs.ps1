[CmdletBinding()]
param(
    [string]$WorktreeRoot = "..\\pcs-worktrees",
    [switch]$Strict
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Format-NativeArgument {
    param(
        [Parameter(Mandatory = $true)][string]$Value
    )
    if ($Value -notmatch '[\s"]') {
        return $Value
    }
    $escaped = $Value -replace '(\\*)"', '$1$1\"'
    $escaped = $escaped -replace '(\\+)$', '$1$1'
    return '"' + $escaped + '"'
}

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$Args,
        [int]$TimeoutSeconds = 8
    )
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "git"
    $startInfo.WorkingDirectory = $RepoPath
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = (($Args | ForEach-Object { Format-NativeArgument -Value "$_" }) -join ' ')

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        try { $process.Kill() } catch {}
        $process.WaitForExit()
        $exitCode = 124
    } else {
        $exitCode = $process.ExitCode
    }
    $stdoutText = $process.StandardOutput.ReadToEnd()
    $stderrText = $process.StandardError.ReadToEnd()
    $stdoutLines = if ($stdoutText) { @($stdoutText -split "`r?`n" | Where-Object { $_ -ne "" }) } else { @() }
    $stderrLines = if ($stderrText) { @($stderrText -split "`r?`n" | Where-Object { $_ -ne "" }) } else { @() }
    if ($exitCode -eq 124) {
        $stderrLines += "git command timed out after $TimeoutSeconds seconds"
    }
    $lines = @($stdoutLines + $stderrLines)
    [pscustomobject]@{
        ExitCode = $exitCode
        Output   = $lines
    }
}

function Get-GitOutput {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$Args
    )
    $result = Invoke-Git -RepoPath $RepoPath -Args $Args -TimeoutSeconds 5
    if ($result.ExitCode -ne 0) { return @() }
    return @($result.Output)
}

function Update-RemoteBranch {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$Branch,
        [int]$MaxAttempts = 3
    )
    $attemptErrors = @()
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        $result = Invoke-Git -RepoPath $RepoPath -Args @("fetch", "origin", $Branch, "--quiet") -TimeoutSeconds 8
        if ($result.ExitCode -eq 0) {
            return [pscustomobject]@{
                Status  = "fresh"
                Warning = $null
                Error   = $null
            }
        }

        $message = ($result.Output | Where-Object { $_ -and $_.Trim() } | ForEach-Object { $_.Trim() }) -join " | "
        if (-not $message) {
            $message = "git fetch exited $($result.ExitCode)"
        }
        $attemptErrors += "attempt ${attempt}: $message"

        if ($attempt -lt $MaxAttempts) {
            Start-Sleep -Milliseconds (400 * $attempt)
        }
    }

    $remoteRefCheck = Invoke-Git -RepoPath $RepoPath -Args @("show-ref", "--verify", "--quiet", "refs/remotes/origin/$Branch") -TimeoutSeconds 5
    if ($remoteRefCheck.ExitCode -eq 0) {
        return [pscustomobject]@{
            Status  = "cached"
            Warning = "remote fetch failed after $MaxAttempts attempts; using cached origin/$Branch reference"
            Error   = $null
        }
    }

    return [pscustomobject]@{
        Status  = "failed"
        Warning = $null
        Error   = "remote fetch failed after $MaxAttempts attempts and no cached origin/$Branch reference is available :: $($attemptErrors -join ' || ')"
    }
}

function Get-StatusValue {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath
    )
    if (-not (Test-Path $FilePath)) { return $null }
    $lines = Get-Content $FilePath
    $idx = $lines.IndexOf("## Status")
    if ($idx -lt 0) { return $null }

    for ($i = $idx + 1; $i -lt [Math]::Min($lines.Count, $idx + 20); $i++) {
        $line = $lines[$i].Trim()
        if ($line -match "^(##|#)\s+") { break }
        if ($line -match "\bready\b") { return "ready" }
        if ($line -match "\bblocked\b") { return "blocked" }
        if ($line -match "\bpending\b") { return "pending" }
    }
    return $null
}

function Assert-FileExists {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label,
        [ref]$Errors
    )
    if (-not (Test-Path $Path)) {
        $Errors.Value += "$Label missing: $Path"
        return $false
    }
    return $true
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\\..")).Path
$commonGitDirRaw = (& git -C $repoRoot rev-parse --git-common-dir 2>$null)
if ($LASTEXITCODE -ne 0 -or -not $commonGitDirRaw) {
    throw "Unable to resolve git common dir for $repoRoot"
}
$commonGitDirText = "$commonGitDirRaw".Trim()
if ([System.IO.Path]::IsPathRooted($commonGitDirText)) {
    $commonGitDir = [System.IO.Path]::GetFullPath($commonGitDirText)
} else {
    $commonGitDir = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $commonGitDirText))
}
$commonRepoRoot = Split-Path -Parent $commonGitDir
$worktreeBase = [System.IO.Path]::GetFullPath((Join-Path $commonRepoRoot $WorktreeRoot))

$modules = @(
    @{
        Name = "api-spec"
        Path = (Join-Path $worktreeBase "api-spec")
        Branch = "module/api-spec"
        RootHandoff = @("api/HANDOFF.md", "spec/HANDOFF.md")
    },
    @{
        Name = "state"
        Path = (Join-Path $worktreeBase "state")
        Branch = "module/state"
        RootHandoff = @("internal/state/HANDOFF.md")
    },
    @{
        Name = "scheduler-evidence"
        Path = (Join-Path $worktreeBase "scheduler-evidence")
        Branch = "module/scheduler-evidence"
        RootHandoff = @("internal/scheduler/HANDOFF.md", "internal/evidence/HANDOFF.md")
    },
    @{
        Name = "verifier-proofs"
        Path = (Join-Path $worktreeBase "verifier-proofs")
        Branch = "module/verifier-proofs"
        RootHandoff = @("verifier/HANDOFF.md", "proofs/HANDOFF.md")
    }
)

$allErrors = @()
$allWarnings = @()

Write-Host "Commander handoff check"
Write-Host "repo: $repoRoot"
Write-Host "worktrees: $worktreeBase"
Write-Host ""

foreach ($m in $modules) {
    $errors = @()
    $warnings = @()
    $path = $m.Path
    Write-Host "== $($m.Name) =="

    if (-not (Test-Path $path)) {
        $errors += "worktree path missing: $path"
    } else {
        $branch = "$((Get-GitOutput -RepoPath $path -Args @('branch', '--show-current') | Select-Object -First 1))".Trim()
        if (-not $branch) { $errors += "not a git repo (or branch unknown): $path" }
        elseif ($branch -ne $m.Branch) { $errors += "wrong branch: expected $($m.Branch) got $branch" }

        $statusResult = Invoke-Git -RepoPath $path -Args @("status", "--porcelain") -TimeoutSeconds 5
        if ($statusResult.ExitCode -ne 0) {
            $message = ($statusResult.Output | Where-Object { $_ -and $_.Trim() } | ForEach-Object { $_.Trim() }) -join " | "
            if (-not $message) {
                $message = "git status exited $($statusResult.ExitCode)"
            }
            $errors += "unable to inspect working tree cleanliness: $message"
        } else {
            $porcelain = @($statusResult.Output | Where-Object { $_ -and $_.Trim() })
            if ($porcelain.Count -gt 0) { $errors += "working tree not clean (commit before claiming done)" }
        }

        $remoteState = Update-RemoteBranch -RepoPath $path -Branch $m.Branch
        if ($remoteState.Status -eq "failed") {
            $errors += $remoteState.Error
        } elseif ($remoteState.Status -eq "cached") {
            $warnings += $remoteState.Warning
        }

        $aheadBehindSpec = "origin/$($m.Branch)...HEAD"
        $aheadBehind = "$((Get-GitOutput -RepoPath $path -Args @('rev-list', '--left-right', '--count', $aheadBehindSpec) | Select-Object -First 1))".Trim()
        if ($aheadBehind) {
            $parts = $aheadBehind -split "\s+"
            if ($parts.Count -ge 2) {
                $behind = [int]$parts[0]
                $ahead = [int]$parts[1]
                if ($behind -gt 0) { $errors += "behind origin by $behind commits (pull/rebase)" }
                if ($ahead -gt 0) { $errors += "ahead of origin by $ahead commits (push required)" }
            }
        }

        foreach ($rel in $m.RootHandoff) {
            $hp = Join-Path $path $rel
            [void](Assert-FileExists -Path $hp -Label "handoff" -Errors ([ref]$errors))
            $status = Get-StatusValue -FilePath $hp
            if (-not $status) { $errors += "handoff status missing/unknown in $rel (expected ready|blocked|pending)" }
            elseif ($Strict -and $status -ne "ready") { $errors += "handoff not ready: $rel ($status)" }
        }
    }

    if ($errors.Count -eq 0) {
        if ($warnings.Count -eq 0) {
            Write-Host "OK"
        } else {
            foreach ($w in $warnings) { Write-Host "WARN: $w" }
            Write-Host "OK (with warnings)"
            $allWarnings += @($warnings | ForEach-Object { "$($m.Name): $_" })
        }
    } else {
        foreach ($w in $warnings) { Write-Host "WARN: $w" }
        foreach ($e in $errors) { Write-Host "FAIL: $e" }
        $allErrors += @($errors | ForEach-Object { "$($m.Name): $_" })
    }
    Write-Host ""
}

if ($allErrors.Count -gt 0) {
    Write-Host "Overall: FAIL"
    exit 2
}

if ($allWarnings.Count -gt 0) {
    Write-Host "Overall: WARN"
    exit 0
}

Write-Host "Overall: PASS"
exit 0
