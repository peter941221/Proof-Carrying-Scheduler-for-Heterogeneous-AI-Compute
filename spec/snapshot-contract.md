# Snapshot Contract

## Purpose

`Snapshot` is the deterministic scheduler input assembled from telemetry, capacity, policy, and topology signals at one logical point in time.

## Required sections

- node inventory
- resource availability
- network topology or latency edges
- policy-relevant labels
- snapshot metadata:
  - `snapshot_id`
  - `snapshot_hash`
  - `version`
  - `snapshot_time`

## Determinism rules

- identical normalized inputs must produce the same snapshot payload and hash
- normalization must resolve missing or malformed optional fields deterministically
- node order and edge order must not change the resulting `snapshot_hash`
- references to unknown nodes, clusters, or fault domains must fail before scheduling

## Hash boundary

The `snapshot_hash` covers the canonical form of the normalized snapshot payload, including:

- node capacities and free capacity
- topology or latency edges used for candidate evaluation
- labels and policy-relevant metadata

The `snapshot_hash` excludes:

- external raw source blobs
- transient ingestion logs
- verifier-side recomputation artifacts

## Relation to `DecisionBundle`

- a bundle binds to one snapshot through `snapshot_ref.snapshot_hash`
- `snapshot_ref.snapshot_version` must match the snapshot payload `version`
- the verifier may require the snapshot payload out of band, but the hash and metadata must match exactly
