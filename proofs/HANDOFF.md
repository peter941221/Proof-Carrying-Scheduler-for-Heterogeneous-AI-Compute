# Proofs Handoff

## Status

- `ready`

## Summary

- Locked a Week-1 minimal formal scope (protocol-level evidence chain + snapshot binding) and kept the current artifact set narrow enough not to block implementation.
- Tightened the proof-side boundary notes so the TLA+ artifacts explicitly exclude freshness and runtime verifier `CHECKED` implications.
- Clarified the Week-1 success criteria so proof artifacts remain honest about what still requires verifier implementation under `verifier/`.
- Preserved the existing claim traceability and assumptions artifacts as the durable source of modeled-vs-planned claim boundaries.

## Files changed

- `proofs/tla+/README.md`
- `proofs/week-1-scope.md`
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
- Owned-doc markdown path check + verifier claim issue/reporting consistency check (PowerShell one-liners)
- result:
- pass

## Risks / follow-ups

- open issues:
- TLC/model-check was not executed in this environment (tooling not assumed installed); artifacts are intended as a clean starting point.
- commander decision needed:
- Confirm whether Week-2 should expand the TLA+ scope toward constraint semantics or keep proofs strictly protocol-level until verifier implementation stabilizes.
