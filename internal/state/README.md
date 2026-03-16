# State Module

This module builds deterministic scheduler snapshots from normalized cluster and workload inputs.

## Goal (week-1)

- produce a normalized snapshot payload that is deterministic and stable under input ordering
- define and enforce unknown-reference failure behavior early (fail before scheduling)
- provide fixtures that cover mixed CPU/GPU clusters and common topology variants

## Owns

- internal snapshot models
- normalization rules
- snapshot assembly
- snapshot hashing support

## Depends on

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/snapshot-contract.md`
- `../../spec/canonical-json.md`
- `README.md`

## Inputs and outputs

- input: telemetry, capacity, topology, labels, and task-facing metadata
- output: deterministic snapshot payload plus `snapshot_id` / `snapshot_hash`

## Contract anchors (must align)

- `../../spec/snapshot-contract.md` (determinism + hash boundary)
- `../../spec/canonical-json.md` (canonical JSON rules)
- `../../api/proto/pcs/v1/scheduler.proto` (`Node`, `NetworkEdge`, `SnapshotMetadata`, `SnapshotRef`)

## Must not change directly

- shared protobuf contracts
- canonical JSON or bundle hash semantics
- scheduler or evidence behavior outside the snapshot interface

## State-owned structures

The state module assembles the JSON form of `SnapshotMetadata` and treats it as the implementation target for hashing and fixtures.

Required top-level fields:

- `snapshotId`
- `version`
- `snapshotTime`
- `nodes[]`
- `networkEdges[]`
- `labels{}`

Hash-covered content is the normalized snapshot payload with `snapshotHash` omitted during hashing and restored afterward.

### Node expectations

Each node entry is the protobuf JSON form of `Node` and must include enough identity to support stable ordering and reference checks:

- required identifiers: `nodeId`, `clusterId`
- stable placement fields: `region`, `zone`
- optional policy/topology fields: `faultDomain`, `spot`, `labels{}`
- resource sections: `capacityTotal`, `capacityFree`

### Edge expectations

Each edge entry is the protobuf JSON form of `NetworkEdge`:

- required references: `srcId`, `dstId`
- optional metrics: `p50LatencyMs`, `p95LatencyMs`, `jitterMs`, `lossRate`, `observedAt`

Edges are part of the snapshot hash boundary whenever they influence candidate evaluation.

## Normalization order (deterministic)

The normalization pipeline must produce a payload that is stable across:

- different input ordering (node/edge arrival order)
- missing optional fields handled deterministically
- map key ordering handled by canonical JSON emission

Recommended deterministic ordering rules before hashing:

- `nodes[]` sorted by:
  1. `clusterId`
  2. `region`
  3. `zone`
  4. `nodeId`
- `networkEdges[]` sorted by:
  1. `srcId`
  2. `dstId`
- maps (`labels`, `ResourceVector.ext`) emitted with lexicographically sorted keys
- absent optional fields omitted instead of serialized as `null`

Arrays should preserve this semantic order into canonical JSON; canonicalization sorts object keys but does not reorder arrays.

## Hash boundary

`snapshotHash` covers the canonical JSON form of the normalized snapshot payload, including:

- node capacities and free capacity
- topology edges used for placement or scoring
- policy-relevant labels and metadata inside the snapshot payload

`snapshotHash` excludes:

- the `snapshotHash` field itself
- external raw ingestion blobs
- transient ingestion logs or recomputation artifacts

## Unknown-reference failure behavior (fail-fast)

Fail snapshot assembly and do not emit `snapshotHash` when:

- a `networkEdges[].srcId` or `networkEdges[].dstId` does not reference a known `nodes[].nodeId`
- required identifiers are missing or empty (`snapshotId`, `nodeId`, `clusterId`)
- structural fields are malformed (for example, `nodes` or `networkEdges` is not an array)

This matches `spec/snapshot-contract.md`: unknown references must fail before scheduling.

## Snapshot hash rule (implementation policy)

Compute `snapshotHash` as follows:

1. assemble the normalized snapshot payload using protobuf JSON field names (`snapshotId`, `networkEdges`, `capacityTotal`, ...)
2. remove `snapshotHash` from the payload to avoid self-reference
3. canonicalize JSON per `spec/canonical-json.md`:
   - sort object keys lexicographically
   - emit maps with sorted keys
   - preserve array order from normalization
   - omit `null` object fields
4. compute `sha256` over the UTF-8 bytes of the canonical JSON
5. encode the result as `sha256:<hex>`

The `sha256:<hex>` prefix matches existing repository examples. If the shared contract standardizes a different algorithm or envelope later, update the README, tooling, and fixtures together.

## Fixtures

Fixtures live in `internal/state/fixtures/` and cover:

- mixed CPU-only and GPU nodes
- multiple regions/zones with topology edges
- label maps and `ResourceVector.ext` map ordering
- invalid unknown-node-reference cases for fail-fast validation

See `internal/state/fixtures/README.md` for fixture intent and local verification commands.
