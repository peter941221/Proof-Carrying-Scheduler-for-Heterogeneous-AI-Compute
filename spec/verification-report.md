# Verification Report Contract

## Purpose

`VerifyResponse` is the independent verifier's structured judgment on one `DecisionBundle`.

## Required booleans

- `valid`
- `signature_valid`
- `constraints_valid`
- `bound_valid`
- `counterfactuals_valid`

These flags must be internally consistent:

- if `valid = true`, the response must not contain any `ISSUE_SEVERITY_ERROR` or `ISSUE_SEVERITY_CRITICAL` issues
- `signature_valid = true` implies signature verification was executed and succeeded; if signature verification was skipped, `signature_valid` must be `false` and a `SIGNATURE.SKIPPED` issue must be emitted
- `constraints_valid = true` implies constraint verification was executed and succeeded; if constraint verification was skipped, `constraints_valid` must be `false` and a `CONSTRAINT.SKIPPED` issue must be emitted

## Issue code families

- `STRUCT.*`
- `HASH.*`
- `SIGNATURE.*`
- `SNAPSHOT.*`
- `CONSTRAINT.*`
- `BOUND.*`
- `COUNTERFACTUAL.*`
- `CLAIM.*`

## Checked claims

`checked_claims` records which formal or engineering claims were actually exercised during verification.

Each claim entry should include:

- `claim_id`
- `status`
- `artifact_refs`
- a short summary

## Minimal verifier behavior

The verifier must:

- report every failing check as at least one structured issue
- avoid silent downgrade from failed checks to warnings
- attach related IDs whenever a task, assignment, or constraint can be pinpointed

## Skipped checks

If a check is disabled by `VerifyRequest`:

- emit an `ISSUE_SEVERITY_INFO` issue indicating the skip (e.g. `SIGNATURE.SKIPPED`)
- set the corresponding `*_valid` flag to `false`
- do not imply the check succeeded via `checked_claims`

If `strict_mode = true`, skipped checks must be treated as `ISSUE_SEVERITY_ERROR` and `valid` must be `false`.
