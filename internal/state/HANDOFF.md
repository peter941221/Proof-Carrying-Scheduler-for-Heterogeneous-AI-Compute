# State Module Handoff (worktree: module/state)

## Status

- ready

## Summary

This worktree completed the State mission: it now includes an implementation-ready snapshot assembly guide, deterministic fixtures, and small local tools for hash and fail-fast verification. Generated Python bytecode was removed from source control and is now ignored inside the module scope.

## Files changed / added (owned scope only)

```text
internal/state/
├─ README.md
├─ HANDOFF.md
├─ .gitignore
├─ fixtures/
│  ├─ README.md
│  ├─ mixed_cpu_gpu.v1.snapshot.json
│  ├─ topology_multi_zone.v1.snapshot.json
│  └─ unknown_node_ref.v1.snapshot.invalid.json
└─ tools/
   ├─ canonical_hash.py
   ├─ validate_snapshot.py
   └─ verify_fixtures.py
```

## Contract decisions captured in `internal/state/README.md`

- State-owned payload shape is the protobuf JSON form of `SnapshotMetadata`.
- Normalization order is explicit before hashing:
  - `nodes[]`: `clusterId`, `region`, `zone`, `nodeId`
  - `networkEdges[]`: `srcId`, `dstId`
  - maps: lexicographically sorted keys; omit absent optionals
- Hash boundary is explicit:
  - include node capacities, edges, and policy-relevant labels/metadata in the snapshot payload
  - exclude `snapshotHash` itself and external transient inputs
- Fail-fast behavior is explicit:
  - unknown edge references fail assembly before hashing/scheduling
  - missing required identifiers and malformed structural arrays also fail assembly
- Snapshot hash policy is explicit:
  - canonical JSON with `snapshotHash` stripped
  - `sha256` over UTF-8 canonical bytes
  - encoded as `sha256:<hex>`

## Fixtures (what they cover)

- `mixed_cpu_gpu.v1.snapshot.json`
  - valid mixed CPU and GPU nodes in one cluster
  - includes an edge so `networkEdges[]` participates in the hash boundary
- `topology_multi_zone.v1.snapshot.json`
  - valid multi-zone and multi-region topology with multiple edges
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

- The repo examples use `sha256:` prefixes, but the shared snapshot contract still does not explicitly standardize the algorithm string.
- `SnapshotMetadata` currently carries the full snapshot payload; if the shared contract later introduces a dedicated `Snapshot` message, the state README and fixtures should be realigned to that type.
