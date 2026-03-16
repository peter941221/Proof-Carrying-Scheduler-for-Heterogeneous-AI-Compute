# Proofs Module

This module holds formal and semi-formal assurance artifacts that justify project claims.

## Owns

- protocol and invariant models
- proof-oriented checker assets
- theorem or model-check artifacts
- claim-to-artifact traceability

## Depends on

- `../spec/claim-taxonomy.md`
- `../spec/claim-lifecycle.md`
- `../spec/decision-bundle.md`
- `../spec/snapshot-contract.md`

## Inputs and outputs

- input: stable claim IDs, protocol semantics, and verifier-facing assumptions
- output: named artifacts that support specific claim IDs and boundaries

## Must not change directly

- implementation contracts in `api/`
- scheduler runtime behavior
- public guarantee wording that exceeds existing artifacts

## First delivery target

- choose the smallest week-1 TLA+ scope
- map initial claim IDs to artifacts or explicit placeholders
- keep proof scope narrow enough that implementation can proceed in parallel

## Working docs (v0)

- scope choice: `week-1-scope.md`
- claim ↔ artifact mapping: `claim-traceability.md`
- TLA+ artifacts: `tla+/README.md`
- model assumptions: `tla+/ASSUMPTIONS.md`
