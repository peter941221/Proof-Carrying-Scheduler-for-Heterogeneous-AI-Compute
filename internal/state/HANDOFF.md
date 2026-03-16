# State Module Handoff (worktree: module/state)

## Status

- ready

## Summary

This worker round tightened the implementation guide for snapshot assembly, strengthened fixture documentation, and upgraded local verification to enforce deterministic node and edge ordering in addition to fail-fast validation and hash checks. One existing topology fixture was normalized to the documented sort order and re-hashed so the fixture set now matches the README contract.

## Files changed / added (owned scope only)

```text
internal/state/
├─ README.md
├─ HANDOFF.md
├─ fixtures/
│  ├─ README.md
│  └─ topology_multi_zone.v1.snapshot.json
└─ tools/
   └─ verify_fixtures.py
```

## Contract decisions captured in `internal/state/README.md`

- State-owned payload shape is the protobuf JSON form of `SnapshotMetadata`.
- Snapshot assembly flow is explicit: project -> validate -> normalize -> omit nulls -> hash -> emit.
- Normalization order is explicit before hashing:
  - `nodes[]`: `clusterId`, `region`, `zone`, `nodeId`
  - `networkEdges[]`: `srcId`, `dstId`
  - maps: lexicographically sorted keys during canonical JSON emission
- Hash boundary is explicit:
  - include node capacities, node labels, snapshot labels, and topology edges/metrics used by scheduling
  - exclude `snapshotHash` itself and transient external inputs
- Fail-fast behavior is explicit:
  - unknown edge references fail assembly before hashing/scheduling
  - missing required identifiers and malformed arrays/object entries also fail assembly

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
python -m py_compile internal/state/tools/canonical_hash.py internal/state/tools/validate_snapshot.py internal/state/tools/verify_fixtures.py
python internal/state/tools/verify_fixtures.py
```

Result:

- PASS (`verify_fixtures.py`): 2 valid fixtures, 1 invalid fixture

## Acceptance criteria check

- README sufficient for implementation: YES
- fixtures cover valid and invalid paths: YES
- contract drift from `spec/snapshot-contract.md`: NO drift detected
- edits made outside `internal/state/`: NO

## Open questions / follow-ups

- The root snapshot contract mentions unknown cluster and fault-domain references, but the current shared payload shape does not yet expose first-class cluster or fault-domain reference tables to validate against.
- The shared contract still implies `SnapshotMetadata`; if a dedicated snapshot message is introduced later, realign this README, fixtures, and tooling together.
