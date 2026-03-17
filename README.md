# Proof-Carrying Scheduler for Heterogeneous AI Compute

Proof-Carrying Scheduler for Heterogeneous AI Compute is a local-first monorepo for a scheduler whose decisions ship with independently verifiable evidence.

## Core idea

- **Decision plane**: admit work, generate candidates, score, optimize, and dispatch.
- **Evidence plane**: emit feasibility, bound, conflict, and fallback certificates for every decision.
- **Independent verifier**: re-check hashes, signatures, and constraints without trusting the scheduler.
- **Formal assurance**: keep TLA+, model-checking, and theorem assets alongside the implementation.

## Planned layout

```text
api/
spec/
internal/state/
internal/scheduler/
internal/evidence/
verifier/
proofs/
scripts/
```

## Multi-agent workflow

This repo is prepared for a commander-and-worktrees workflow:

- shared contracts live at the repo root and in `api/` + `spec/`
- implementation modules work in isolated Git worktrees
- verifier and proof work stay separate from the scheduler hot path
- local-only coordination docs are intentionally ignored because the intended GitHub repo is public
- the tracked orchestration framework now lives in `LangGraph-Commander/` and reads repo settings from `commander.toml`

## Documentation layering

- `README.md` explains the repo-wide model and worktree workflow
- `api/README.md` and `spec/README.md` define the shared contract layer
- each implementation module has its own local README so a module worktree feels like a small project root
- shared semantics still have one source of truth; module READMEs summarize and point back to the root contracts instead of redefining them

## Task and handoff flow

- each public module directory contains a `TASK.md` with the full current mission for that module
- each module also contains a `HANDOFF.md` that the worker fills in before asking for review
- workers should complete the whole `TASK.md` in one pass when feasible, then stop and submit the filled `HANDOFF.md`
- the commander reviews handoffs, accepts, or issues targeted rework

## Local bootstrap

Use the PowerShell helpers to bootstrap a fresh local clone or recreate the multi-worktree setup:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/bootstrap-local-repo.ps1
powershell -ExecutionPolicy Bypass -File scripts/new-module-worktree.ps1 -Module state
```

## LangGraph-Commander

The reusable Rust control plane now lives under `LangGraph-Commander/`.

Common commands:

```powershell
commander
commander status
commander check
```

Notes:

- `commander` must be started by Peter from the project root before Codex can issue `start <worker>` commands
- the panel is now the required live monitor and emergency-stop console; closing it stops all running workers
- default launch opens the Ratatui dashboard inline in the current terminal
- `status`, `brief`, and `check` also work as one-shot commands when no live dashboard is running
- `start <worker>` is hard-gated behind a live panel heartbeat and prints a fixed prompt if Peter has not opened `commander` yet
- `stop <worker>` and `stop all` remain available as emergency controls
