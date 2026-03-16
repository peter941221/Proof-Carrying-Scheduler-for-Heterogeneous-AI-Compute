# Proofs Handoff

## Status

- `ready`

## Summary

- Locked a Week-1 minimal formal scope (protocol-level evidence chain + snapshot binding) and added initial durable artifacts:
  - scope doc + claim↔artifact traceability
  - TLA+ skeleton model for append-only chain linkage and decision_id uniqueness
  - placeholder artifacts for deferred claims to keep filenames stable
- Strengthened the Week-1 TLA+ asset by making the immediate-predecessor linkage boundary explicit in the assumptions artifact.
- Tightened claim traceability notes so each MODELED or PLANNED claim states the exact current boundary of the supporting artifact set, including snapshot freshness and ancestry-proof exclusions.
- Sharpened append-only semantics by making append the only modeled transition and reflecting that boundary in the scope and traceability docs.
- Added a durable assumptions artifact so the abstract hash/snapshot boundary is explicit and reusable by future proof work.

## Files changed

- `proofs/week-1-scope.md`
- `proofs/claim-traceability.md`
- `proofs/tla+/ASSUMPTIONS.md`
- `proofs/HANDOFF.md`

## Claim coverage

- modeled claims:
- `EVIDENCE.APPEND_ONLY_CHAIN`
- `SAFETY.UNIQUE_BINDING`
- `SAFETY.SNAPSHOT_CONSISTENCY`
- planned claims:
- `SAFETY.RESOURCE_CAPACITY`
- `VERIFY.BUNDLE_SOUNDNESS`
- `VERIFY.CONFLICT_SOUNDNESS`
- `BOUND.RELATIVE_GAP_REPORTING`
- `LIVENESS.CONDITIONAL_COMPLETION`
- deferred claims:
- Any optimization/solver optimality claims
- Full constraint replay semantics
- Signature / PKI trust policy modeling

## Validation run

- command:
- Owned-doc markdown path check + proofs claim traceability coverage check (PowerShell one-liners)
- result:
- pass

## Risks / follow-ups

- open issues:
- TLC/model-check was not executed in this environment (tooling not assumed installed); artifacts are intended as a clean starting point.
- commander decision needed:
- Confirm whether Week-2 should expand the TLA+ scope toward constraint semantics or keep proofs strictly protocol-level until verifier implementation stabilizes.
