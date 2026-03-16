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
