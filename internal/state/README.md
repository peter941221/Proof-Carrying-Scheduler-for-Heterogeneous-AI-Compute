# State Module

This module builds deterministic scheduler snapshots from normalized cluster and workload inputs.

## Owns

- internal snapshot models
- normalization rules
- snapshot assembly
- snapshot hashing support

## Depends on

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/snapshot-contract.md`
- `../../spec/contract-packet.md`

## Inputs and outputs

- input: telemetry, capacity, topology, labels, and task-facing metadata
- output: deterministic internal snapshot structures plus `snapshot_id` / `snapshot_hash`

## Must not change directly

- shared protobuf contracts
- canonical JSON or bundle hash semantics
- scheduler or evidence behavior outside the snapshot interface

## First delivery target

- document internal `Node`, `Task`, and `Snapshot` shapes
- define normalization order
- define unknown-reference failure behavior
- prepare fixture coverage for mixed CPU / GPU snapshots
