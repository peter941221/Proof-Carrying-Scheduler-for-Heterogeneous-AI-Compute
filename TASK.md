# Integration Task Entry

This worktree runs the `integration/m1` mission.

## What to do

1. Merge completed module branches in dependency order:
   - `module/api-spec`
   - `module/state`
   - `module/scheduler-evidence`
   - `module/verifier-proofs`
2. Preserve filled module handoffs and contract docs instead of reverting them to templates.
3. Resolve cross-module conflicts without changing shared semantics unless required.
4. Run targeted integration validation before asking for review.

## Handoff rule

The integration branch is ready for review only when merged module handoffs still match reality and the integration worktree is clean.
