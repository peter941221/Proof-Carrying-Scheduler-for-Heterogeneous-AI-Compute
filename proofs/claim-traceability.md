# Claim ↔ Artifact Traceability

This file tracks claim IDs (from `spec/claim-taxonomy.md`) to concrete artifacts and their lifecycle status (from `spec/claim-lifecycle.md`).

The intent is to keep claim language honest: each claim's status must not exceed what artifacts actually justify.

## Claim table (initial)

```text
SAFETY.RESOURCE_CAPACITY
  status: PLANNED
  artifacts:
    - proofs/placeholders/SAFETY.RESOURCE_CAPACITY.md
  notes: no Week-1 formal semantics for resource feasibility yet

SAFETY.UNIQUE_BINDING
  status: MODELED
  artifacts:
    - proofs/tla+/EvidenceChain.tla
    - proofs/tla+/README.md
    - proofs/tla+/ASSUMPTIONS.md
  notes: covers decision_id uniqueness within the append-only evidence log only

SAFETY.SNAPSHOT_CONSISTENCY
  status: MODELED
  artifacts:
    - proofs/tla+/EvidenceChain.tla
    - proofs/tla+/README.md
    - proofs/tla+/ASSUMPTIONS.md
  notes: covers per-entry binding to exactly one abstract snapshot_hash, not snapshot payload semantics or freshness

EVIDENCE.APPEND_ONLY_CHAIN
  status: MODELED
  artifacts:
    - proofs/tla+/EvidenceChain.tla
    - proofs/tla+/README.md
    - proofs/tla+/ASSUMPTIONS.md
  notes: covers append-only immediate-predecessor linkage because append is the only modeled transition, not cryptographic collision resistance or ancestry proofs

VERIFY.BUNDLE_SOUNDNESS
  status: PLANNED
  artifacts:
    - verifier/verification-stages.md
    - verifier/issue-codes.md
    - verifier/claim-reporting.md
    - proofs/placeholders/VERIFY.BUNDLE_SOUNDNESS.md
  notes: becomes CHECKED once verifier implementation exists and exercises S0-S2

VERIFY.CONFLICT_SOUNDNESS
  status: PLANNED
  artifacts:
    - verifier/verification-stages.md
    - verifier/claim-reporting.md
    - proofs/placeholders/VERIFY.CONFLICT_SOUNDNESS.md
  notes: becomes CHECKED once constraint replay exists for conflict bundles

BOUND.RELATIVE_GAP_REPORTING
  status: PLANNED
  artifacts:
    - proofs/placeholders/BOUND.RELATIVE_GAP_REPORTING.md
  notes: no Week-1 bound semantics or runtime checker artifact yet

LIVENESS.CONDITIONAL_COMPLETION
  status: PLANNED
  artifacts:
    - proofs/placeholders/LIVENESS.CONDITIONAL_COMPLETION.md
  notes: current model includes only minimal append progress, not conditional completion semantics
```

## Update rule

When a new durable artifact is added (model-check output, proof, replay harness), update the corresponding claim entry with:

- the strongest status actually justified
- stable artifact references (repo-relative paths)
- a one-line boundary statement (what is and is not covered)
