# Claim Taxonomy

## Claim families

- `SAFETY.*`
- `EVIDENCE.*`
- `VERIFY.*`
- `BOUND.*`
- `LIVENESS.*`

## Initial claim IDs

- `SAFETY.RESOURCE_CAPACITY`
- `SAFETY.UNIQUE_BINDING`
- `SAFETY.SNAPSHOT_CONSISTENCY`
- `EVIDENCE.APPEND_ONLY_CHAIN`
- `VERIFY.BUNDLE_SOUNDNESS`
- `VERIFY.CONFLICT_SOUNDNESS`
- `BOUND.RELATIVE_GAP_REPORTING`
- `LIVENESS.CONDITIONAL_COMPLETION`

## Artifact reference format

Artifact references in `ClaimCheck.artifact_refs` are opaque strings, but producers should use stable, type-prefixed handles so downstream tooling can route them without guessing.

Recommended placeholder format:

- `<artifact_type>:<stable_id>`

Examples:

- `proof:bundle-soundness-v1`
- `report:verify-decision-20260317-001`
- `log:solver-trace-sha256-abc123`
- `model:tla-snapshot-consistency-v1`

## Initial family placeholders

Each initial claim family should have at least one placeholder artifact reference reserved, even before a durable artifact exists.

- `SAFETY.*`: `model:safety-placeholder`
- `EVIDENCE.*`: `log:evidence-placeholder`
- `VERIFY.*`: `report:verify-placeholder`
- `BOUND.*`: `report:bound-placeholder`
- `LIVENESS.*`: `model:liveness-placeholder`

These placeholders mark the intended artifact channel only. They do not justify a stronger `ClaimStatus` than the current lifecycle evidence supports.

## Usage rules

- every proof artifact or verifier report should reference one or more claim IDs
- claims may be marked using the shared `ClaimStatus` vocabulary:
  - `CLAIM_STATUS_PLANNED`
  - `CLAIM_STATUS_MODELED`
  - `CLAIM_STATUS_CHECKED`
  - `CLAIM_STATUS_IMPLEMENTED`
  - `CLAIM_STATUS_VERIFIED`
- `ClaimCheck.status` records the strongest status actually supported by the referenced artifacts at the stated boundary
- no document should imply a stronger guarantee than the current claim status supports
