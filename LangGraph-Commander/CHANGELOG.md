# LangGraph-Commander Changelog

## 1.2.0

- Replace the old game-menu panel model with a fixed four-quadrant TUI.
- Make the lower-right command console the default interaction surface with inline editing and history.
- Add local-only panel commands for `view ...`, `show worker ...`, and `filter ...` without mutating supervisor state.
- Expand scroll handling so live output, worker orchestration, and command logs all stay readable under larger fleets.
- Lean harder into the LazyGit-inspired hardcore ASCII look while keeping the supervisor command surface compatible.
- Add worker model/reasoning badges in the orchestration pane and switch the local live feed to tail-lock on by default.

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
