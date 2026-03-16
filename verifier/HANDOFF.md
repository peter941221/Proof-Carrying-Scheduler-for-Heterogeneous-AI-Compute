# Verifier Handoff

## Status

- `ready`

## Summary

- Delivered v0 verifier policy docs that make `VerifyResponse` derivation deterministic:
  - stage order + gating + hard-stop/soft-fail rules
  - stable issue-code catalog aligned with `spec/verification-report.md` families
  - explicit checked-claim reporting policy that prevents lifecycle overstatement
- Tightened claim synthesis guidance so `checked_claims` are emitted only after final stage outcomes are known, including `blocked` handling after hard-stop failures.
- Added a stage outcome table that gives future implementations a direct path from stage result to response flags and claim-synthesis inputs.
- Added a durable claim matrix that maps each current verifier-facing claim ID to required stages, downgrade behavior, and stable `artifact_refs` guidance.

## Files changed

- `verifier/verification-stages.md`
- `verifier/issue-codes.md`
- `verifier/claim-reporting.md`
- `verifier/claim-matrix.md`
- `verifier/README.md`

## Stage coverage

- mandatory stages:
- S0-S2 (parse, structural validation, canonical JSON + bundle_hash recomputation)
- optional stages:
- S3-S7 gated by `VerifyRequest` (`verify_signature`, `verify_constraints`, `verify_bound`, `verify_counterfactuals`)
- synthesis stage:
- S8 derives `checked_claims` only after enabled stages resolve to `passed`, `failed`, `skipped`, or `blocked`
- implementation guide:
- `verifier/verification-stages.md` includes a stage outcome table and `verifier/claim-matrix.md` maps claim IDs to required stages, downgrade rules, and stable artifact refs
- skipped-stage policy:
- disabled stages must set the corresponding boolean flag to `false` and emit an `INFO` issue (`*.SKIPPED` / `CLAIM.SKIPPED_STAGE`) to avoid omission being mistaken for success

## Validation run

- command:
- Markdown relative-link check across owned docs + verifier claim-matrix coverage check (PowerShell one-liner)
- result:
- pass

## Risks / follow-ups

- open issues:
- Spec does not yet define sub-codes under each issue family; current catalog is in `verifier/issue-codes.md` and can be promoted to spec later if desired.
- Tooling not enforced: `buf lint` not run here (buf not installed).
- commander decision needed:
- Whether to standardize verifier sub-codes in `spec/verification-report.md` (recommended once multiple implementations exist).
