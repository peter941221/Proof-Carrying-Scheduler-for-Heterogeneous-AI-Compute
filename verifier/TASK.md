# Verifier Task

## Mission

Define the independent verification path so a future implementation can produce stable issue codes, flags, and claim reporting without trusting scheduler internals.

## Owned paths

- `verifier/`

## Required inputs

- `../api/proto/pcs/v1/scheduler.proto`
- `../spec/verification-report.md`
- `../spec/decision-bundle.md`
- `../spec/claim-taxonomy.md`
- `../spec/claim-lifecycle.md`

## Work packages

### 1. Lock verification stage order

- document stage sequence, prerequisites, and hard-stop vs soft-fail behavior

### 2. Lock issue-code policy

- define stable issue families and representative codes
- make skipped-stage semantics explicit

### 3. Lock claim-reporting policy

- define how `checked_claims` are populated
- prevent runtime verification from overstating lifecycle status

## Acceptance criteria

- verifier docs explain how to derive `VerifyResponse` deterministically
- issue-code families align with `spec/verification-report.md`
- claim reporting never claims stronger than runtime checks justify
- no edits are made outside `verifier/`

## Forbidden changes

- no direct edits to `proofs/`
- no scheduler-side behavioral rules
- no API field changes without commander approval

## Required handoff

Fill `verifier/HANDOFF.md` with summary, files, stage coverage, validation, and any spec gaps found.
