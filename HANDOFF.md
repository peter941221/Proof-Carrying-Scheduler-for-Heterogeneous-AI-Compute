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
- Scope completed: added `internal/state/tools/normalize_snapshot.py`, updated `internal/state/tools/verify_fixtures.py` to reuse normalization logic, and documented the normalizer workflow in `internal/state/fixtures/README.md`.
- Validation: `python -m py_compile internal/state/tools/canonical_hash.py internal/state/tools/validate_snapshot.py internal/state/tools/normalize_snapshot.py internal/state/tools/verify_fixtures.py`; `python internal/state/tools/verify_fixtures.py`; `python internal/state/tools/normalize_snapshot.py --in internal/state/fixtures/topology_multi_zone.v1.snapshot.json --out -`
- See `internal/state/HANDOFF.md` for the detailed handoff.
