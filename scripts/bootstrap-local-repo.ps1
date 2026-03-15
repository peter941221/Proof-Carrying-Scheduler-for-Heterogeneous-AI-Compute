[CmdletBinding()]
param(
    [string]$RemoteUrl = "https://github.com/peter941221/Proof-Carrying-Scheduler-for-Heterogeneous-AI-Compute.git",
    [switch]$SetRemote
)

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $repoRoot
try {
    $nestedGitDir = Join-Path $repoRoot ".git"
    $hasLocalRepo = Test-Path $nestedGitDir

    $currentTopLevel = $null
    try {
        $currentTopLevel = (git rev-parse --show-toplevel 2>$null).Trim()
    } catch {
        $currentTopLevel = $null
    }

    if (-not $hasLocalRepo) {
        if ($currentTopLevel -and ((Resolve-Path $currentTopLevel).Path -ne $repoRoot)) {
            Write-Warning "This directory is currently nested inside another Git repo: $currentTopLevel"
            Write-Warning "Initializing here will create an independent project repo for this scheduler."
        }

        git init --initial-branch main | Out-Host
        if ($LASTEXITCODE -ne 0) {
            git init | Out-Host
            if ($LASTEXITCODE -ne 0) {
                throw "git init failed."
            }

            $currentBranch = (git branch --show-current).Trim()
            if ($currentBranch -and $currentBranch -ne "main") {
                git branch -m main | Out-Host
                if ($LASTEXITCODE -ne 0) {
                    throw "Failed to rename the initial branch to main."
                }
            }
        }
    } else {
        Write-Host "Local Git repo already exists at $repoRoot"
    }

    if ($SetRemote) {
        $remotes = @()
        try {
            $remotes = @(git remote)
        } catch {
            $remotes = @()
        }

        if ($remotes -contains "origin") {
            git remote set-url origin $RemoteUrl | Out-Host
        } else {
            git remote add origin $RemoteUrl | Out-Host
        }

        if ($LASTEXITCODE -ne 0) {
            throw "Failed to configure origin remote."
        }
    }

    Write-Host ""
    Write-Host "Repository root: $repoRoot"
    Write-Host "Next actions:"
    Write-Host "  1. git add ."
    Write-Host "  2. git commit -m 'Bootstrap repository coordination scaffold'"
    Write-Host "  3. git branch integration/m1"
    Write-Host "  4. powershell -ExecutionPolicy Bypass -File scripts/new-module-worktree.ps1 -Module api-spec"
    Write-Host "  5. Repeat for state, scheduler-evidence, verifier-proofs"
}
finally {
    Pop-Location
}
