# TLA+ Artifacts

## EvidenceChain

`EvidenceChain.tla` models a minimal append-only evidence log with:

- `decision_id` uniqueness
- chain linkage via `prev_bundle_hash` -> prior `bundle_hash`
- snapshot binding as an abstract field (`snapshot_hash`)
- a single-step transition relation that only permits append actions
- append progress as a minimal liveness target (`<>(Len(log) > 0)`)

### Invariants in the current spec

- `Inv_UniqueDecisionIds`: no two log entries share a `decision_id`
- `Inv_ChainLink`: every append links to the immediately prior `bundle_hash`
- `Inv_SnapshotBinding`: each log entry binds to exactly one abstract `snapshot_hash`

### Boundary notes

- The current model sharpens append-only semantics by making `Append` the only allowed state transition.
- The explicit modeling assumptions are documented in `ASSUMPTIONS.md`.
- It does not model cryptographic collision resistance, snapshot payload contents, or any scheduler optimization semantics.

### Running (optional)

If you have TLC installed, you can model-check the defaults in `EvidenceChain.cfg` by opening the spec in the TLA+ Toolbox or running TLC in your environment.
