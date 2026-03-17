# LangGraph-Commander Changelog

## 1.2.0

- Replace the old game-menu panel model with a fixed four-quadrant TUI.
- Make the lower-right command console the default interaction surface with inline editing and history.
- Add local-only panel commands for `view ...`, `show worker ...`, and `filter ...` without mutating supervisor state.
- Expand scroll handling so live output, worker orchestration, and command logs all stay readable under larger fleets.
- Lean harder into the LazyGit-inspired hardcore ASCII look while keeping the supervisor command surface compatible.
- Add worker model/reasoning badges in the orchestration pane and switch the local live feed to tail-lock on by default.
- Compact consecutive duplicate activity lines into repeat counts so patrol noise no longer overwhelms more useful worker output.
- Mark paused live-feed scroll state directly in the title and make `Ctrl+F` toggle the effective local follow state after manual scrolling.
- Emit explicit start markers and elapsed-time summaries for long-running coordination commands so `intake`, `review`, and `report` are easier to monitor.
- Drop the hard live-panel gate for `start`; headless CLI runs can now spin up a temporary supervisor session when needed.
- Add global `--stream` support so the CLI can print a compact fleet line plus realtime worker / coordination progress in the current terminal.
- Persist per-worker `current_activity` and bridge LangGraph custom stream progress through the Python worker bridge into the Rust runtime feed.
- Focus targeted `--stream start <worker>` output on the requested worker set and surface plain worker `stdout` as compact terminal progress lines.
- Suppress duplicate timestamped bridge telemetry in targeted streams and harden runtime state writes with atomic replace + retry handling for Windows cross-process monitoring.

## 1.1.0

- Add game-style TUI navigation with explicit `NAV`, `ACT`, and `CMD` modes.
- Promote arrows and `WASD` to primary pane selection, with `Enter` or `E` to engage a pane.
- Add fast ops hotkeys for follow-tail, density, stream scope, refresh, patrol, and pane jumps.
- Refresh help text, header badges, and panel styling to make the focused pane and mode more obvious.

## 1.0.0

- Introduce the first tracked Rust workspace for LangGraph-Commander.
- Add a Windows-first Cargo workspace with CLI, supervisor, TUI, provider, and core crates.
- Add a tracked `commander.toml` project manifest for reusable phase/worker orchestration.
- Replace Python-first startup assumptions with a Rust-first control-plane foundation.
