# Verifier Handoff

## Status

- `ready`

## Summary

- Delivered v0 verifier policy docs that make `VerifyResponse` derivation deterministic:
  - stage order + gating + hard-stop/soft-fail rules
  - stable issue-code catalog aligned with `spec/verification-report.md` families
  - explicit checked-claim reporting policy that prevents lifecycle overstatement
- Tightened blocked-stage handling so verifier issue emission distinguishes operator-disabled `skipped` stages from prerequisite-lost `blocked` stages.
- Added `CLAIM.BLOCKED_STAGE` guidance to keep `checked_claims` downgrade reasoning explicit without mislabeling blocked stages as skipped.
- Preserved the deterministic stage outcome table and claim matrix as the implementation path from stage results to response flags and claim entries.

## Files changed

- `verifier/verification-stages.md`
- `verifier/issue-codes.md`
- `verifier/claim-reporting.md`
- `verifier/HANDOFF.md`

## Stage coverage

- mandatory stages:
- S0-S2 (parse, structural validation, canonical JSON + bundle_hash recomputation)
- optional stages:
- S3-S7 gated by `VerifyRequest` (`verify_signature`, `verify_constraints`, `verify_bound`, `verify_counterfactuals`)
- synthesis stage:
- S8 derives `checked_claims` only after enabled stages resolve to `passed`, `failed`, `skipped`, or `blocked`
- implementation guide:
- `verifier/verification-stages.md` includes stage-local rules for skipped versus blocked issue emission, and `verifier/claim-reporting.md` defines how those outcomes downgrade claims deterministically
- skipped-stage policy:
- disabled stages must set the corresponding boolean flag to `false` and emit an `INFO` issue (`*.SKIPPED` / `CLAIM.SKIPPED_STAGE`) to avoid omission being mistaken for success
- blocked-stage policy:
- prerequisite failures must not be relabeled as skipped; claim downgrade reasoning may use `CLAIM.BLOCKED_STAGE` when the earlier hard-stop alone is not sufficiently explanatory

## Validation run

- command:
- Owned-doc markdown path check + verifier claim issue/reporting consistency check (PowerShell one-liners)
- result:
- pass

## Risks / follow-ups

- open issues:
- Spec does not yet define sub-codes under each issue family; current catalog is in `verifier/issue-codes.md` and can be promoted to spec later if desired.
- Tooling not enforced: `buf lint` not run here (buf not installed).
- commander decision needed:
- Whether to standardize verifier sub-codes in `spec/verification-report.md` (recommended once multiple implementations exist).
