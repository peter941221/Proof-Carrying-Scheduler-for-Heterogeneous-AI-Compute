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
- Scope completed: strengthened `internal/state/README.md`, expanded fixture notes, added ordering checks to `internal/state/tools/verify_fixtures.py`, and normalized `internal/state/fixtures/topology_multi_zone.v1.snapshot.json` to the documented sort order.
- Validation: `python -m py_compile internal/state/tools/canonical_hash.py internal/state/tools/validate_snapshot.py internal/state/tools/verify_fixtures.py`; `python internal/state/tools/verify_fixtures.py`
- See `internal/state/HANDOFF.md` for the detailed handoff.
