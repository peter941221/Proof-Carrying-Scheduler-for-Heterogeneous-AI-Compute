# Verifier Module

This module independently checks scheduler evidence without trusting the scheduler implementation.

## Owns

- bundle parsing and structure checks
- hash and signature verification
- constraint replay checks
- structured verification reporting

## Depends on

- `../api/proto/pcs/v1/scheduler.proto`
- `../spec/decision-bundle.md`
- `../spec/canonical-json.md`
- `../spec/verification-report.md`
- `../spec/claim-taxonomy.md`

## Inputs and outputs

- input: `DecisionBundle` plus the referenced snapshot payload
- output: `VerifyResponse`, issue codes, and checked-claim reporting

## Must not change directly

- scheduler-side scoring logic
- proof claim names without shared review
- API compatibility guarantees once published

## First delivery target

- define the verification stage order
- map each issue family to concrete failure cases
- make claim reporting explicit instead of implied

## Working docs (v0)

- stage order and flag semantics: `verification-stages.md`
- issue code catalog: `issue-codes.md`
- checked-claim reporting policy: `claim-reporting.md`
- claim-to-stage matrix: `claim-matrix.md`
