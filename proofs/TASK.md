# Proofs Task

## Mission

Define the smallest honest proof asset set that supports early claims without blocking engineering implementation.

## Owned paths

- `proofs/`

## Required inputs

- `../spec/claim-taxonomy.md`
- `../spec/claim-lifecycle.md`
- `../spec/decision-bundle.md`
- `../spec/snapshot-contract.md`
- `../verifier/README.md`

## Work packages

### 1. Lock week-1 proof scope

- choose a minimal but valuable first formal slice
- keep optimization and full constraint replay out of scope unless absolutely required

### 2. Build claim-to-artifact traceability

- map each initial claim to a current artifact or explicit placeholder
- keep status honest using the shared lifecycle vocabulary

### 3. Add first proof assets or placeholders

- create small, named artifacts that future proof work can grow from
- ensure filenames are durable and correspond to traceability refs

## Acceptance criteria

- every initial claim has either an artifact or an explicit placeholder path
- proof scope is narrow enough not to block scheduler/verifier implementation
- no document overstates claim status
- no edits are made outside `proofs/`

## Forbidden changes

- no rewriting API or verifier contracts
- no optimistic “verified” language without a durable artifact

## Required handoff

Fill `proofs/HANDOFF.md` with summary, files, claim coverage, validation, and any claims intentionally deferred.
