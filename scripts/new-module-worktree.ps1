[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("api-spec", "state", "scheduler-evidence", "verifier-proofs")]
    [string]$Module,

    [string]$BaseBranch = "main",
    [string]$WorktreeRoot = "..\\pcs-worktrees"
)

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

$branchMap = @{
    "api-spec" = "module/api-spec"
    "state" = "module/state"
    "scheduler-evidence" = "module/scheduler-evidence"
    "verifier-proofs" = "module/verifier-proofs"
}

$ownedPathMap = @{
    "api-spec" = @("api/", "spec/")
    "state" = @("internal/state/")
    "scheduler-evidence" = @("internal/scheduler/", "internal/evidence/")
    "verifier-proofs" = @("verifier/", "proofs/")
}

Push-Location $repoRoot
try {
    if (-not (Test-Path (Join-Path $repoRoot ".git"))) {
        throw "This directory is not its own Git repo yet. Run scripts/bootstrap-local-repo.ps1 first."
    }

    $topLevel = (git rev-parse --show-toplevel).Trim()
    if ((Resolve-Path $topLevel).Path -ne $repoRoot) {
        throw "The current Git top-level is $topLevel. Finish standalone repo bootstrap before creating worktrees."
    }

    $branchName = $branchMap[$Module]
    $worktreeBase = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $WorktreeRoot))
    $targetPath = Join-Path $worktreeBase $Module

    if (-not (Test-Path $worktreeBase)) {
        New-Item -ItemType Directory -Path $worktreeBase | Out-Null
    }

    if (Test-Path $targetPath) {
        throw "Target worktree path already exists: $targetPath"
    }

    $existingBranchOutput = @(git branch --list $branchName 2>$null)
    $existingBranch = ($existingBranchOutput -join "").Trim()
    if ([string]::IsNullOrWhiteSpace($existingBranch)) {
        git worktree add $targetPath -b $branchName $BaseBranch | Out-Host
    } else {
        git worktree add $targetPath $branchName | Out-Host
    }

    if ($LASTEXITCODE -ne 0) {
        throw "git worktree add failed."
    }

    Write-Host ""
    Write-Host "Module: $Module"
    Write-Host "Branch: $branchName"
    Write-Host "Path:   $targetPath"
    Write-Host "Owned paths:"
    foreach ($ownedPath in $ownedPathMap[$Module]) {
        Write-Host "  - $ownedPath"
    }
}
finally {
    Pop-Location
}
