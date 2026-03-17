# Worktree Task Entry

This worktree runs the **State** mission.

## What to do

1. Read and execute:
   - `internal/state/TASK.md`
2. Make changes only under:
   - `internal/state/`
3. When finished, fill in:
   - `internal/state/HANDOFF.md`

## Latest round

- Status: ready
- Scope completed: documented optional internal `clusters[]` / `faultDomains[]` anti-ghost reference tables, extended `internal/state/tools/validate_snapshot.py`, updated `internal/state/tools/normalize_snapshot.py`, and added `internal/state/fixtures/unknown_fault_domain_ref.v1.snapshot.invalid.json`.
- Validation: `python internal/state/tools/verify_fixtures.py`; `python internal/state/tools/normalize_snapshot.py --in internal/state/fixtures/unknown_fault_domain_ref.v1.snapshot.invalid.json --out -`
- See `internal/state/HANDOFF.md` for the detailed handoff.
