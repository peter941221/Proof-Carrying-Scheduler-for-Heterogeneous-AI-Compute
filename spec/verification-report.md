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
- if `signature_valid = false`, then `valid` must be `false`
- if `constraints_valid = false`, then `valid` must be `false`
- if a check is skipped and `strict_mode = false`, the corresponding `*_valid` flag remains `true` but the skip must still be surfaced in `issues`

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

`status` must not overstate the achieved boundary; use the shared lifecycle meanings from `spec/claim-lifecycle.md`.

## Minimal verifier behavior

The verifier must:

- report every failing check as at least one structured issue
- avoid silent downgrade from failed checks to warnings
- attach related IDs whenever a task, assignment, or constraint can be pinpointed
- emit skip issues for any disabled check, even when the overall result remains valid

## VerifyRequest toggles

`VerifyRequest` may disable certain checks (e.g. `verify_signature = false`).

Contract rule:
- if a check is disabled, the corresponding `*_valid` flag must be `true` and the verifier should emit an `ISSUE_SEVERITY_INFO` issue indicating the check was skipped (e.g. `SIGNATURE.SKIPPED`).
- if `strict_mode = true`, skipped checks must be reported as `ISSUE_SEVERITY_ERROR` and `valid` must be `false`.
