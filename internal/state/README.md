# State Module

This module assembles deterministic scheduler snapshots from normalized cluster and workload inputs.

## Goal

- define the exact snapshot payload shape the state module owns
- make normalization rules implementation-ready so input arrival order cannot change results
- fail fast on bad references before scheduling or hashing
- provide fixtures and local tooling that lock the contract into deterministic examples

## Owns

- internal snapshot models and assembly flow
- pre-hash normalization rules
- snapshot payload hashing support
- fail-fast structural/reference validation
- local fixtures that exercise valid and invalid paths

## Depends on

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/snapshot-contract.md`
- `../../spec/canonical-json.md`
- `README.md`

## Contract anchors

Use these as the source of truth when implementing the module:

- `../../spec/snapshot-contract.md`: required sections, determinism rules, fail-fast behavior
- `../../spec/canonical-json.md`: canonical JSON rules and omitted/null handling
- `../../api/proto/pcs/v1/scheduler.proto`: protobuf field names and JSON names for `Node`, `NetworkEdge`, `ResourceVector`, and `SnapshotMetadata`

This README is the implementation guide for the state worktree; it summarizes those contracts without changing them.

## Output shape

The implementation target is the protobuf JSON form of `SnapshotMetadata`.

Required top-level fields:

- `snapshotId`
- `version`
- `snapshotTime`
- `nodes[]`
- `networkEdges[]`
- `labels{}`

Optional top-level fields:

- `snapshotHash`
- `clusters[]`
- `faultDomains[]`

The state module should assemble a complete normalized snapshot object, compute `snapshotHash` over the hash-covered content, then write the hash back into the emitted payload.

`clusters[]` and `faultDomains[]` are internal planning aids in this worktree, not frozen shared wire-contract fields. They exist only to document and locally validate anti-ghost-resource semantics when upstream inputs include explicit reference tables.

## State-owned structures

### `nodes[]`

Each entry is the protobuf JSON form of `Node`.

Required identity fields:

- `nodeId`
- `clusterId`

Stable placement fields used by normalization and topology-aware scheduling:

- `region`
- `zone`

Common optional policy/topology fields:

- `faultDomain`
- `spot`
- `labels{}`
- `observedAt`
- pricing / carbon / risk / health fields

Resource sections:

- `capacityTotal`
- `capacityFree`

Each resource section is the protobuf JSON form of `ResourceVector`, including optional `ext{}` for implementation-specific metrics.

### `networkEdges[]`

Each entry is the protobuf JSON form of `NetworkEdge`.

Required reference fields:

- `srcId`
- `dstId`

Optional edge metrics:

- `p50LatencyMs`
- `p95LatencyMs`
- `jitterMs`
- `lossRate`
- `observedAt`

Edges are inside the snapshot hash boundary whenever they affect candidate generation, placement feasibility, or scoring.

### `labels{}`

Top-level labels hold snapshot-wide policy-relevant metadata.

Implementation guidance:

- keep only labels that affect placement, filtering, or policy interpretation
- use protobuf JSON field names exactly
- emit object keys in canonical order through canonical JSON, not by preserving ingestion order

### Optional internal reference tables

The coordination packet requires explicit ghost-resource semantics for clusters and fault domains. The shared snapshot contract does not currently expose first-class reference messages, so this worktree models them only as optional internal validation inputs:

- `clusters[]` entries with `clusterId`
- `faultDomains[]` entries with `faultDomainId`

If these arrays are present during assembly or fixture validation:

- every `nodes[].clusterId` must reference `clusters[].clusterId`
- every non-empty `nodes[].faultDomain` must reference `faultDomains[].faultDomainId`
- unknown references fail assembly before hashing

If these arrays are absent, node and edge validation still runs, but cluster/fault-domain existence cannot be checked from the frozen shared payload shape alone.

## Assembly flow

Implement snapshot assembly in this order:

1. ingest raw node, resource, topology, and label inputs
2. project them into protobuf-JSON-shaped snapshot objects
3. validate required identifiers and reference integrity
4. normalize arrays and maps into deterministic semantic order
5. omit absent optional fields instead of serializing `null`
6. remove `snapshotHash` from the object before hashing
7. canonicalize JSON and compute `sha256:<hex>`
8. restore `snapshotHash` and emit the final payload

If step 3 fails, stop immediately and do not emit a hash.

## Normalization order

Canonical JSON sorts object keys, but it does not reorder arrays. The state module must therefore normalize semantic array order before hashing.

### Node ordering

Sort `nodes[]` by these keys, in order:

1. `clusterId`
2. `region`
3. `zone`
4. `nodeId`

Tie-break behavior:

- compare missing values as empty strings
- do not depend on ingestion order once these keys are available

### Edge ordering

Sort `networkEdges[]` by these keys, in order:

1. `srcId`
2. `dstId`

If future contracts add edge identity beyond `(srcId, dstId)`, extend ordering consistently and update fixtures with the README.

### Internal reference table ordering

If optional internal reference tables are present, sort them before hashing so the same logical inventory cannot hash differently due to ingestion order:

- `clusters[]` by `clusterId`
- `faultDomains[]` by `faultDomainId`

### Map ordering

Canonical JSON handles object key ordering, but implementations should still treat these as unordered maps:

- top-level `labels`
- `Node.labels`
- `ResourceVector.ext`

Behavioral rules:

- object keys are emitted lexicographically during canonicalization
- missing maps may be omitted
- explicit `null` map values are not serialized

### Optional fields

Normalize optional fields deterministically:

- omit absent fields instead of emitting `null`
- preserve explicit boolean `false` and numeric `0`
- keep empty strings only when they are semantically intentional in the produced payload

## Hash boundary

`snapshotHash` covers the canonical JSON form of the normalized snapshot payload, including:

- node capacities and free capacity
- node labels and policy-relevant node metadata
- snapshot-wide labels and metadata that affect scheduling behavior
- topology edges and edge metrics used for placement or scoring
- optional internal reference tables if they are present during assembly and used to validate anti-ghost-resource constraints

`snapshotHash` excludes:

- the `snapshotHash` field itself
- external ingestion blobs
- transient ingestion logs
- recomputation artifacts outside the emitted snapshot payload

## Snapshot hash rule

Compute `snapshotHash` exactly as follows:

1. assemble the normalized snapshot payload using protobuf JSON field names such as `snapshotId`, `networkEdges`, `capacityTotal`, and `capacityFree`
2. strip `snapshotHash` everywhere before hashing
3. canonicalize JSON per `../../spec/canonical-json.md`
4. hash the UTF-8 bytes of the canonical JSON with SHA-256
5. encode the result as `sha256:<hex>`

The current repo examples use the `sha256:` prefix, so the fixtures and helper tools enforce that representation.

## Fail-fast behavior for unknown references

Fail snapshot assembly before hashing and before scheduling when any of the following occurs:

- `networkEdges[].srcId` does not match a known `nodes[].nodeId`
- `networkEdges[].dstId` does not match a known `nodes[].nodeId`
- `nodes[].clusterId` does not match a known `clusters[].clusterId` when `clusters[]` is present
- `nodes[].faultDomain` does not match a known `faultDomains[].faultDomainId` when `faultDomains[]` is present
- required identifiers are missing or empty, including `snapshotId`, `nodeId`, or `clusterId`
- `nodes` is present but is not an array
- `networkEdges` is present but is not an array
- a node, edge, cluster, or fault-domain entry is present but is not an object

This keeps the worktree aligned with `spec/snapshot-contract.md` while staying inside the current shared-contract freeze: ghost cluster and fault-domain checks are implemented only when local inputs provide explicit reference tables.

## Deterministic implementation notes

When coding the real assembler:

- normalize first, hash second; never hash raw ingestion order
- keep validation separate from canonicalization so failures are easy to explain
- compute the hash from a copy of the snapshot object with `snapshotHash` removed
- use protobuf JSON names, not proto snake_case names
- if local reference tables are used for anti-ghost checks, normalize and hash them consistently with the rest of the payload
- keep fixture verification in sync with any contract-tightening change

## Fixtures

Fixtures live in `internal/state/fixtures/`.

- `mixed_cpu_gpu.v1.snapshot.json`: valid mixed CPU/GPU snapshot with one edge
- `topology_multi_zone.v1.snapshot.json`: valid topology-aware multi-zone snapshot with multiple edges and map-ordering coverage
- `unknown_node_ref.v1.snapshot.invalid.json`: invalid snapshot where an edge references an unknown node and assembly must fail
- `unknown_fault_domain_ref.v1.snapshot.invalid.json`: invalid snapshot where a node references a fault domain missing from the optional internal reference table

See `internal/state/fixtures/README.md` for intent and local verification commands.
