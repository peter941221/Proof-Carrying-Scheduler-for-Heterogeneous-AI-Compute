# Spec Task

## Mission

Finish the human-readable contract packet so each module can implement and validate behavior without redefining shared semantics.

## Owned paths

- `spec/`

## Required inputs

- `README.md`
- `api/proto/pcs/v1/scheduler.proto`

## Work packages

### 1. Close semantic gaps in shared contracts

- refine `decision-bundle`, `snapshot-contract`, `verification-report`, `claim-taxonomy`, and `claim-lifecycle` where a module could reasonably misinterpret behavior
- prefer short normative statements over long explanations

### 2. Keep root truth singular

- align terminology across all spec docs
- ensure one authoritative definition exists for each of:
  - snapshot binding
  - bundle hashing
  - fallback semantics
  - verification result semantics
  - claim status semantics

### 3. Improve example alignment

- update spec examples only where necessary to match the frozen contract
- avoid adding example complexity unless another module truly needs it

## Acceptance criteria

- state, scheduler/evidence, and verifier/proofs modules can cite `spec/` as the single truth source
- no spec doc contradicts protobuf field semantics
- claim wording never exceeds current proof/verifier boundaries
- no edits are made outside `spec/`

## Forbidden changes

- no implementation pseudocode that belongs in module docs
- no cross-module policy decisions hidden only in examples

## Required handoff

Fill `spec/HANDOFF.md` with summary, changed files, validation, conflicts resolved, and any remaining ambiguity.
