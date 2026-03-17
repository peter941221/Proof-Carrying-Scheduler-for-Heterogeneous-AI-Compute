# Week-1 Proof Scope (TLA+ Minimal Slice)

This document chooses the smallest Week-1 formal scope that still supports useful early claims without blocking implementation.

## Objective

Model the **evidence append-only chain** and **bundle↔snapshot binding** at the protocol level, independent of scheduler optimization details.

## In-scope (Week-1)

- An append-only evidence log of `DecisionBundle`-like records
- Chain linkage via `prev_bundle_hash` -> prior `bundle_hash`
- Uniqueness of `decision_id` within the log
- Snapshot binding: every bundle references exactly one `snapshot_hash`
- Deterministic hashing function treated as an abstract function (no crypto modeling)
- A transition system whose only state change is append
- An explicit assumptions file that records the abstraction boundary for Week-1 claims

## Out-of-scope (defer)

- Solver optimality, scoring, and dispatch semantics
- Full constraint replay semantics (resource, latency, policy, etc.)
- Signature and PKI trust policy
- Counterfactual and bound certificate math

## Claims supported by this scope

Initial mapping to `spec/claim-taxonomy.md`:

- `EVIDENCE.APPEND_ONLY_CHAIN` (target: `MODELED` in Week-1)
- `SAFETY.UNIQUE_BINDING` (target: `MODELED` for decision_id uniqueness)
- `SAFETY.SNAPSHOT_CONSISTENCY` (target: `MODELED` for bundle↔snapshot binding under abstract snapshot identifiers)

Other claims remain `PLANNED` until additional semantics are modeled or checked.

## Artifact plan

```text
proofs/tla+/EvidenceChain.tla     The model (log, linkage, uniqueness)
proofs/tla+/EvidenceChain.cfg     Default TLC model config (minimal)
proofs/tla+/ASSUMPTIONS.md        Explicit abstraction boundary for Week-1
proofs/claim-traceability.md      Claim -> artifact refs + status
```

## Success criteria (Week-1)

- the TLA+ model compiles in common tooling (TLC)
- invariants for chain linkage, uniqueness, and snapshot binding are stated
- append-only semantics are represented directly in the transition relation
- assumptions that limit claim strength are recorded in a durable artifact
- at least one simple progress condition is stated (log grows by append)
- the proof artifacts do not imply verifier runtime `CHECKED` coverage for claims that still require implementation
