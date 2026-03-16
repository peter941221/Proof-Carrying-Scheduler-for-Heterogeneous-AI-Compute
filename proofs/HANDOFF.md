# Proofs Handoff

## Status

- `ready`

## Summary

- Locked a Week-1 minimal formal scope (protocol-level evidence chain + snapshot binding) and added initial durable artifacts:
  - scope doc + claim↔artifact traceability
  - TLA+ skeleton model for append-only chain linkage and decision_id uniqueness
  - placeholder artifacts for deferred claims to keep filenames stable
- Strengthened the Week-1 TLA+ asset by stating the explicit snapshot-binding invariant and a minimal append-progress property in the artifact docs.
- Tightened claim traceability notes so each MODELED or PLANNED claim states the exact current boundary of the supporting artifact set.
- Sharpened append-only semantics by making append the only modeled transition and reflecting that boundary in the scope and traceability docs.
- Added a durable assumptions artifact so the abstract hash/snapshot boundary is explicit and reusable by future proof work.

## Files changed

- `proofs/week-1-scope.md`
- `proofs/claim-traceability.md`
- `proofs/tla+/EvidenceChain.tla`
- `proofs/tla+/EvidenceChain.cfg`
- `proofs/tla+/README.md`
- `proofs/tla+/ASSUMPTIONS.md`
- `proofs/placeholders/SAFETY.RESOURCE_CAPACITY.md`
- `proofs/placeholders/VERIFY.BUNDLE_SOUNDNESS.md`
- `proofs/placeholders/VERIFY.CONFLICT_SOUNDNESS.md`
- `proofs/placeholders/BOUND.RELATIVE_GAP_REPORTING.md`
- `proofs/placeholders/LIVENESS.CONDITIONAL_COMPLETION.md`
- `proofs/README.md`

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
- Markdown relative-link check across owned docs + proof placeholder existence check + claim traceability coverage check + TLA config invariant coverage check + assumptions file presence check (PowerShell one-liner)
- result:
- pass

## Risks / follow-ups

- open issues:
- TLC/model-check was not executed in this environment (tooling not assumed installed); artifacts are intended as a clean starting point.
- commander decision needed:
- Confirm whether Week-2 should expand the TLA+ scope toward constraint semantics or keep proofs strictly protocol-level until verifier implementation stabilizes.
