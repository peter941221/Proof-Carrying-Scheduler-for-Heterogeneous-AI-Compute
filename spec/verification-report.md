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

- if `valid = true`, no `ISSUE_SEVERITY_ERROR` or `ISSUE_SEVERITY_CRITICAL` issue may remain unresolved
- if `signature_valid = false`, then `valid` must be `false`
- if `constraints_valid = false`, then `valid` must be `false`

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
