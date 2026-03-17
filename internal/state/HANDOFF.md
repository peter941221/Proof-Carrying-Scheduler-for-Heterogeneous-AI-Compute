# State Module Handoff (worktree: module/state)

## Status

- ready

## Summary

This worker round closed the coordination-packet gap around ghost resources by documenting optional internal `clusters[]` and `faultDomains[]` reference tables, extending validation to enforce those references when present, and adding an invalid ghost-fault-domain fixture. The normalizer now also sorts those optional reference tables so anti-ghost validation inputs stay deterministic before hashing.

## Files changed / added (owned scope only)

```text
internal/state/
├─ README.md
├─ HANDOFF.md
├─ fixtures/
│  ├─ README.md
│  └─ unknown_fault_domain_ref.v1.snapshot.invalid.json
└─ tools/
   ├─ normalize_snapshot.py
   └─ validate_snapshot.py
```

## Contract decisions captured in `internal/state/README.md`

- The frozen shared contract still centers on protobuf-JSON `SnapshotMetadata` fields.
- Optional internal `clusters[]` and `faultDomains[]` tables are documented only as local validation inputs, not as new shared wire fields.
- When those tables are present:
  - `nodes[].clusterId` must match `clusters[].clusterId`
  - `nodes[].faultDomain` must match `faultDomains[].faultDomainId`
  - unknown references fail before hashing and scheduling
- If those tables are used during assembly, they participate in deterministic normalization and in the local snapshot hash boundary.

## Hash-participating normalized fields

The local state plan now names these hash-participating fields explicitly:

- top-level: `snapshotId`, `version`, `snapshotTime`, `labels`, `nodes`, `networkEdges`
- node content: `nodeId`, `clusterId`, `region`, `zone`, `faultDomain`, `spot`, `labels`, `capacityTotal`, `capacityFree`, and policy-relevant optional node metadata when present
- edge content: `srcId`, `dstId`, `p50LatencyMs`, `p95LatencyMs`, `jitterMs`, `lossRate`, `observedAt` when present
- optional internal reference tables: `clusters[]` and `faultDomains[]` when present and used for anti-ghost validation
- excluded: `snapshotHash`, external ingestion blobs, transient logs, and recomputation artifacts outside the emitted payload

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
- `unknown_fault_domain_ref.v1.snapshot.invalid.json`
  - invalid fixture where a node references an undeclared fault domain from the optional internal reference table
  - defines deterministic ghost-resource failure semantics without changing frozen shared contracts

## Validation (local)

Commands run from repo root:

```powershell
python internal/state/tools/verify_fixtures.py
python internal/state/tools/normalize_snapshot.py --in internal/state/fixtures/unknown_fault_domain_ref.v1.snapshot.invalid.json --out -
```

Result:

- PASS (`verify_fixtures.py`): 2 valid fixtures, 2 invalid fixtures
- PASS (`normalize_snapshot.py`): emits deterministically ordered JSON for the ghost-fault-domain invalid fixture

## Acceptance criteria check

- README sufficient for implementation: YES
- fixtures cover valid and invalid paths: YES
- helper tooling stays scoped to normalization/hash verification: YES
- no contract drift from `spec/snapshot-contract.md`: YES
- ghost-resource semantics documented and locally validated: YES
- edits made outside `internal/state/`: NO

## Open questions / follow-ups

- Ghost cluster validation is implemented in tooling when optional `clusters[]` are provided, but there is not yet a dedicated invalid cluster fixture; add one only if future rounds still need more explicit coverage.
- If the shared contract later gains first-class cluster or fault-domain reference messages, replace the local optional tables with the frozen shared shape and realign fixtures/tooling together.
