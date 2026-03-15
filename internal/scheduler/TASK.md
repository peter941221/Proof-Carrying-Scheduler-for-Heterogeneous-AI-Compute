# Scheduler Task

## Mission

Define the hot-path scheduling behavior tightly enough that implementation can begin without hidden heuristics or unstable replay semantics.

## Owned paths

- `internal/scheduler/`

## Required inputs

- `../../api/proto/pcs/v1/scheduler.proto`
- `../../spec/decision-bundle.md`
- `../../spec/snapshot-contract.md`
- `../evidence/README.md`

## Work packages

### 1. Lock stage order

- document the end-to-end decision flow from request intake to evidence handoff
- make deterministic ordering and tie-breaking explicit

### 2. Lock candidate semantics

- define one training path and one inference path that another engineer can implement directly
- ensure every selection-relevant factor can be surfaced to evidence via bundle fields

### 3. Lock scoring semantics

- define objective term order, normalization, contribution calculation, and tie-break rules
- ensure omitted terms are handled consistently

### 4. Lock fallback triggers

- document when fallback must be emitted
- make partial and infeasible paths explicit

## Acceptance criteria

- scheduler docs explain how to produce replayable `Candidate` and `Assignment` payloads
- no hidden heuristic is required to justify a decision
- fallback behavior is explicit enough for evidence and verifier modules to consume
- no edits are made outside `internal/scheduler/`

## Forbidden changes

- no edits to hash/signature semantics
- no verifier policy changes
- no proof-claim rewriting

## Required handoff

Fill `internal/scheduler/HANDOFF.md` with summary, files, decision-path coverage, validation, and dependencies on evidence/spec.
