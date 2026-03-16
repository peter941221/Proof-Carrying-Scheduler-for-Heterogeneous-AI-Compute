# Spec Handoff

## Status

- `ready`

## Summary

- Tightened normative wording in the shared contract docs so bundle status, skipped verification checks, and claim status reporting stay singular across modules.
- Kept examples and frozen surfaces aligned; no new contract surface was introduced.

## Files changed

- `spec/claim-taxonomy.md`
- `spec/decision-bundle.md`
- `spec/verification-report.md`

## Validation run

- command: `python -m json.tool spec/examples/decision-bundle.minimal.json > $null`
- result: pass

## Resolved ambiguities

- `DECISION_STATUS_PARTIAL` now explicitly covers degraded decisions, fallback paths, and policy-approved relaxations, with required `fallback` plus relevant `constraint_evals`.
- Skipped verifier checks are now explicitly required to surface as issues even when `strict_mode = false` and the corresponding `*_valid` flag remains `true`.
- `ClaimCheck.status` is now explicitly tied to the strongest status justified by the referenced artifacts at the stated boundary.

## Risks / follow-ups

- open issues:
- commander decision needed:
  - Confirm whether the signer policy may choose between signing `bundle_hash` and signing canonical bytes, or whether the contract should collapse to a single required signature mode.
