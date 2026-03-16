[CmdletBinding()]
param(
    [string]$WorktreeRoot = "..\\pcs-worktrees",
    [switch]$Strict
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-GitOutput {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string[]]$Args
    )
    $out = & git -C $RepoPath @Args 2>$null
    if ($LASTEXITCODE -ne 0) { return @() }
    if ($null -eq $out) { return @() }
    if ($out -is [string]) { return @($out) }
    return @($out)
}

function Update-RemoteBranch {
    param(
        [Parameter(Mandatory = $true)][string]$RepoPath,
        [Parameter(Mandatory = $true)][string]$Branch
    )
    & git -C $RepoPath fetch origin $Branch --quiet 2>$null
    $null = $LASTEXITCODE
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

Write-Host "Commander handoff check"
Write-Host "repo: $repoRoot"
Write-Host "worktrees: $worktreeBase"
Write-Host ""

foreach ($m in $modules) {
    $errors = @()
    $path = $m.Path
    Write-Host "== $($m.Name) =="

    if (-not (Test-Path $path)) {
        $errors += "worktree path missing: $path"
    } else {
        $branch = (Get-GitOutput -RepoPath $path -Args @("branch", "--show-current") | Select-Object -First 1).Trim()
        if (-not $branch) { $errors += "not a git repo (or branch unknown): $path" }
        elseif ($branch -ne $m.Branch) { $errors += "wrong branch: expected $($m.Branch) got $branch" }

        $porcelain = @(Get-GitOutput -RepoPath $path -Args @("status", "--porcelain"))
        if ($porcelain.Count -gt 0) { $errors += "working tree not clean (commit before claiming done)" }

        Update-RemoteBranch -RepoPath $path -Branch $m.Branch

        $aheadBehind = (Get-GitOutput -RepoPath $path -Args @("rev-list", "--left-right", "--count", "origin/$($m.Branch)...HEAD") | Select-Object -First 1).Trim()
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
        Write-Host "OK"
    } else {
        foreach ($e in $errors) { Write-Host "FAIL: $e" }
        $allErrors += @($errors | ForEach-Object { "$($m.Name): $_" })
    }
    Write-Host ""
}

if ($allErrors.Count -gt 0) {
    Write-Host "Overall: FAIL"
    exit 2
}

Write-Host "Overall: PASS"
exit 0
