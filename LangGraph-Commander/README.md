# LangGraph-Commander

`LangGraph-Commander` is a reusable Rust orchestration framework for multi-agent project work.

## V1.2.0 goals

- Windows-first Cargo workspace
- fast startup and remote control
- LazyGit-inspired terminal UI
- fixed four-quadrant command deck
- command-first workflow with a permanent lower-right input box
- local view/filter commands layered on top of supervisor commands
- reusable project manifest via `commander.toml`
- tracked versions and changelog for every framework change

## Workspace

```text
LangGraph-Commander/
├─ VERSION
├─ CHANGELOG.md
├─ Cargo.toml
└─ crates/
   ├─ lgc-cli/
   ├─ lgc-core/
   ├─ lgc-supervisor/
   ├─ lgc-tui/
   └─ lgc-provider-openai/
```

## Runtime outputs

Runtime files are written under `LangGraph-Commander/runtime/` and are intentionally ignored by Git.

Key files:

- `status.json`
- `assistant-brief.md`
- `patrol-status.json`
- `coordination/state.json`
- `coordination/events.jsonl`
- `remote/control.json`
- `threads/<worker>/state.json`

## Current project integration

This repo configures LangGraph-Commander through the root `commander.toml`.

## Current command surface

Use the PowerShell wrapper from the repo root:

```powershell
commander
commander status
commander brief
commander check
commander intake
commander approve
commander review all
commander report
```

The default mode runs the Rust Ratatui dashboard in the current terminal.

Headless-first monitoring contract:

- `commander` remains the richest live monitor, but it is no longer required before `start`
- `commander --stream start <worker>` opens a temporary supervisor session when needed, prints one-line worker progress in the current terminal, and keeps the fleet line focused on the requested worker set
- `commander --stream intake|approve|review|report` streams coordination subprocess output line-by-line instead of waiting for one final blob
- if a live panel is already running, `--stream` tails the shared runtime while the remote command executes
- closing the panel still stops the workers owned by that panel session
- runtime state files now use atomic replace writes with short retries so headless streaming stays stable under Windows cross-process read/write pressure

Supported commands:

- `tui`
- `open`
- `status`
- `brief`
- `refresh`
- `check`
- `intake`
- `approve`
- `review [all|<worker>]`
- `report`
- `ping [text]`
- `start all|<worker>`
- `stop all|<worker>`
- `patrol [start|stop|once|status]`
- `phase <name>`
- `command <free text>`
- `version`

Global flags:

- `--stream` prints live activity and fleet updates while the command runs
- `--require-running` fails fast unless a live shared commander session already exists

## Panel layout

The TUI is now a denser LazyGit-inspired fixed grid with four persistent quadrants:

- top left: `Command Tower` for phase/session/activation summary plus the quick command deck
- bottom left: `Live Output` for the large scrollable worker and commander stream
- top right: `Worker Orchestration` for the filtered fleet map, active-worker summary, and model/reasoning badges
- bottom right: `Command Input` for the always-hot input line, command log, and shortcuts

Key controls:

- the lower-right input box is always live; type raw supervisor commands directly and press `Enter`
- `Up` / `Down` browse command history, `Left` / `Right` edit inline, and `Space` is normal text again
- local panel commands: `help local`, `view all`, `view selected`, `view commander`, `view worker <name>`, `show worker <name>`, `filter <text>`, `filter clear`
- `PgUp` / `PgDn` scroll live output, `Alt+PgUp` / `Alt+PgDn` scroll the worker roster, and `Alt+Up` / `Alt+Down` scroll the command log
- live output now defaults to local tail-lock on, so new worker output stays pinned to the bottom until you manually scroll away
- `Ctrl+R` refreshes, `Ctrl+S` stops all workers, `Ctrl+D` toggles density, `Ctrl+F` toggles follow-tail
- `Ctrl+N` / `Ctrl+P` cycle the local active worker without mutating shared supervisor state
- `F1` prints local panel help, `F2` jumps the local feed to `view all`, `F3` jumps to `view selected`, and `F4` is emergency `stop all`
- `Esc` twice within 2 seconds exits the panel; session shutdown still stops supervised workers

## Streamed terminal output

When `--stream` is enabled, the CLI keeps the current terminal as the primary realtime surface:

- worker progress prints as compact lines such as `[13:09:10] state | reading TASK.md`
- plain worker `stdout` now also surfaces as compact live lines, so generic `launch_command` workers can report what they are doing without a custom bridge
- coordination commands print stage output as it arrives instead of only at the end
- a `[fleet] ...` line updates when worker status or current activity changes, and targeted `start <worker>` streams keep that snapshot scoped to the requested worker set
- targeted worker streams suppress raw timestamped bridge telemetry when a clearer progress line already exists, so the terminal stays readable during real runs
- worker progress is sourced from real LangGraph custom stream events plus worker state updates, not only inferred after the run

## V1 boundaries

V1 ships the reusable Rust control plane, runtime snapshots, remote inbox/ack flow, audit/patrol loop, and the dense command-first terminal UI.

Worker process launch is intentionally generic:

- if a worker has `launch_command`, the supervisor can spawn and track it
- if `launch_command` is missing, the dashboard marks that worker as `manual-activation`
- this keeps the framework reusable across projects instead of hard-coding PCS-specific Python worker logic back into the new Rust rewrite

## Coordination loop

The project now includes a document-first LangGraph coordination loop layered on top of the supervisor commands:

- `commander intake` reads `tech_plan.txt`, `proof_plan.txt`, module charters, and local task seeds, then writes:
  - `docs/coordination/project-brief.md`
  - `docs/coordination/task-graph.md`
  - `docs/coordination/approval-summary.md`
  - `docs/coordination/escalations.md`
  - `docs/coordination/packets/<worker>.md`
- `commander approve` clears the DAG approval gate for workers that are not frozen by an escalation
- `commander review [all|<worker>]` runs the independent review layer and writes `docs/coordination/reviews/*.md`
- `commander report` refreshes `docs/coordination/review-report.md` and `docs/coordination/retro.md`

Worker launches respect the coordination gate:

- workers stay blocked while their `pending_action` says the DAG still needs approval
- escalated workers stay frozen until Peter resolves the question
- unrelated workers can still move when only part of the graph is frozen
