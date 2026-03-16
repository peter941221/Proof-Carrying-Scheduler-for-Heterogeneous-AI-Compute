# EvidenceChain Assumptions

This file records the explicit abstractions behind `proofs/tla+/EvidenceChain.tla`.

## Week-1 assumptions

- `bundle_hash` values are modeled as abstract elements of `Hashes`; the model does not prove cryptographic collision resistance.
- `snapshot_hash` values are modeled as abstract elements of `SnapshotHashes`; the model does not inspect snapshot payload contents.
- The only state transition is append, so the model excludes rewrite, deletion, and in-place mutation by construction.
- `decision_id` uniqueness is scoped to the modeled log, not to any external federation of logs.
- `prev_bundle_hash` is required to point to the immediately prior log entry's `bundle_hash`; the model does not justify weaker ancestry-only linkage claims.
- Scheduler optimization, constraint satisfaction, signatures, and certificate math are intentionally out of scope.

## What these assumptions justify

Under these assumptions, the Week-1 model can honestly support:

- append-only chain linkage over abstract hashes
- unique `decision_id` binding within one modeled log
- per-entry binding to one abstract snapshot identifier

## What they do not justify

These assumptions do not justify:

- cryptographic soundness claims
- snapshot payload correctness or freshness
- ancestry proofs weaker than immediate predecessor linkage
- assignment feasibility or conflict semantics
- bound, counterfactual, or liveness completion guarantees
