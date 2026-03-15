# Evidence Module

This module turns scheduler decisions into replayable and independently checkable evidence.

## Owns

- constraint witness generation
- bundle construction
- bound / fallback payload assembly
- evidence-ledger integration points

## Depends on

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/decision-bundle.md`
- `../../spec/canonical-json.md`
- `../../spec/verification-report.md`

## Inputs and outputs

- input: chosen assignments, rejected candidates, snapshot references, and solver results
- output: `DecisionBundle` payloads and verification-ready witness data

## Must not change directly

- shared API field semantics
- signer policy shape without commander review
- proof or verifier claims outside evidence scope

## First delivery target

- define minimum `ConstraintEval` coverage for feasible decisions
- define fallback payload behavior for degraded or infeasible paths
- ensure every emitted field can be consumed by the verifier without scheduler-only state
