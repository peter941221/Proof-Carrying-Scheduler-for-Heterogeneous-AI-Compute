# State Module Handoff (worktree: module/state)

## Status

- ready

## Summary

This worker round added a small snapshot normalizer utility for future implementation and fixture authoring, then wired fixture verification to reuse that normalization logic when checking documented array ordering. The fixture docs now include the normalizer workflow so authors can reorder payloads before computing `snapshotHash`.

## Files changed / added (owned scope only)

```text
internal/state/
├─ HANDOFF.md
├─ fixtures/
│  └─ README.md
└─ tools/
   ├─ normalize_snapshot.py
   └─ verify_fixtures.py
```

## Contract decisions captured in `internal/state/README.md`

- No contract drift this round; the README contract remains the implementation source for payload shape, normalization order, hash boundary, and fail-fast behavior.
- Tooling now directly reflects the documented normalization rules by reusing one helper for node and edge ordering.

## Fixtures (what they cover)

- `mixed_cpu_gpu.v1.snapshot.json`
  - valid mixed CPU and GPU nodes in one cluster
  - includes an edge so `networkEdges[]` participates in the hash boundary
- `topology_multi_zone.v1.snapshot.json`
  - valid multi-zone and multi-region topology with deterministic node and edge ordering
  - includes `ResourceVector.ext` map ordering coverage
- `unknown_node_ref.v1.snapshot.invalid.json`
  - invalid fixture where an edge references an unknown node id
  - defines the required fail-fast behavior before hashing/scheduling

## Validation (local)

Commands run from repo root:

```powershell
python -m py_compile internal/state/tools/canonical_hash.py internal/state/tools/validate_snapshot.py internal/state/tools/normalize_snapshot.py internal/state/tools/verify_fixtures.py
python internal/state/tools/verify_fixtures.py
python internal/state/tools/normalize_snapshot.py --in internal/state/fixtures/topology_multi_zone.v1.snapshot.json --out -
```

Result:

- PASS (`verify_fixtures.py`): 2 valid fixtures, 1 invalid fixture
- PASS (`normalize_snapshot.py`): emits deterministically ordered JSON for a valid fixture

## Acceptance criteria check

- README sufficient for implementation: YES
- fixtures cover valid and invalid paths: YES
- helper tooling stays scoped to normalization/hash verification: YES
- contract drift from `spec/snapshot-contract.md`: NO drift detected
- edits made outside `internal/state/`: NO

## Open questions / follow-ups

- The root snapshot contract mentions unknown cluster and fault-domain references, but the current shared payload shape does not yet expose first-class cluster or fault-domain reference tables to validate against.
- If a future implementation wants the helper to recompute `snapshotHash` after normalization, add that as a separate utility or explicit flag rather than silently mutating fixture hashes.
