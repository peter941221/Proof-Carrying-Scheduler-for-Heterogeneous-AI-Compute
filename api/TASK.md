# API Task

## Mission

Finish the public wire-contract layer for the scheduler and verifier so other modules can implement without guessing field semantics.

## Owned paths

- `api/`

## Required inputs

- `README.md`
- `spec/contract-packet.md`
- `spec/decision-bundle.md`
- `spec/verification-report.md`
- `spec/claim-taxonomy.md`

## Work packages

### 1. Tighten protobuf comments and intent

- add concise comments to non-obvious fields in `proto/pcs/v1/scheduler.proto`
- prefer comments that affect interoperability, replay, hash/signature semantics, or verifier behavior
- avoid implementation comments that belong in Go or Rust code

### 2. Freeze verification-facing wire semantics

- ensure `VerificationIssue`, `ClaimCheck`, `SnapshotRef`, and `DecisionBundle` fields are named and documented clearly enough for the verifier and proofs modules
- if a field meaning is still ambiguous, clarify in comments without renaming unless strictly necessary
- do not add fields unless they close a real contract gap

### 3. Add one contract-oriented example if needed

- extend or adjust API-facing examples only if it helps another module implement against the wire format
- keep examples minimal and aligned with `spec/examples/decision-bundle.minimal.json`

## Acceptance criteria

- another engineer can read `scheduler.proto` and understand hash/signature-sensitive fields without opening chat logs
- no duplicate or competing semantics are introduced relative to `spec/`
- field comments do not contradict any root contract doc
- no changes are made outside `api/`

## Forbidden changes

- no edits under `internal/`, `verifier/`, or `proofs/`
- no silent breaking renames after downstream modules already depend on a field
- no new claim wording invented inside proto comments

## Required handoff

Fill `api/HANDOFF.md` with:

- summary of changes
- exact files changed
- validation run
- unresolved ambiguities or commander decisions needed
