# Evidence Task

## Mission

Define how scheduling outputs become replayable, signer-ready, verifier-consumable `DecisionBundle` evidence.

## Owned paths

- `internal/evidence/`

## Required inputs

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/decision-bundle.md`
- `../../spec/canonical-json.md`
- `../../spec/verification-report.md`
- `../scheduler/README.md`

## Work packages

### 1. Lock constraint witness coverage

- define the minimum `ConstraintEval` set for feasible decisions
- define how partial and infeasible paths expose conflict evidence

### 2. Lock bundle assembly rules

- document deterministic assembly, referential integrity checks, hash/signature preparation, and finalization order

### 3. Lock fallback and optional certificate behavior

- define how fallback, bound, counterfactual, and shadow-level fields are populated or intentionally omitted

## Acceptance criteria

- evidence docs are sufficient for a future builder implementation
- every emitted field can be justified from scheduler output plus snapshot data
- no hidden scheduler-only state is required by the verifier
- no edits are made outside `internal/evidence/`

## Forbidden changes

- no direct changes to scheduler scoring
- no verifier issue policy changes
- no root contract drift

## Required handoff

Fill `internal/evidence/HANDOFF.md` with summary, files, witness coverage, validation, and any missing upstream inputs.
