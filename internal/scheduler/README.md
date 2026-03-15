# Scheduler Module

This module owns the hot-path decision plane.

## Owns

- admission logic
- candidate generation
- fast-path scoring
- dispatch selection
- optimizer handoff boundaries

## Depends on

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/decision-bundle.md`
- `../../spec/snapshot-contract.md`

## Inputs and outputs

- input: normalized snapshot plus scheduling requests
- output: candidates, scored decisions, assignments, and fallback context for evidence generation

## Must not change directly

- bundle hash / signature semantics
- verifier report semantics
- proof claim wording

## First delivery target

- define one training-path candidate flow
- define one inference-path candidate flow
- define deterministic score term ordering
- define explicit fallback triggers
