# State Task

## Mission

Turn the snapshot contract into a deterministic implementation-ready state plan with fixtures and local tooling notes.

## Owned paths

- `internal/state/`

## Required inputs

- `../../spec/snapshot-contract.md`
- `../../spec/canonical-json.md`
- `../../api/proto/pcs/v1/scheduler.proto`
- `README.md`

## Work packages

### 1. Finish the module contract doc

- strengthen `README.md` until it fully explains:
  - state-owned structures
  - normalization order
  - hash boundary
  - fail-fast behavior for unknown references

### 2. Produce fixture set

- create minimal deterministic fixtures under `fixtures/`
- include at least:
  - one valid mixed CPU/GPU snapshot
  - one topology-aware snapshot with edges
  - one invalid snapshot that should fail assembly
- each fixture must make clear what behavior it is testing

### 3. Produce lightweight helper notes or tooling

- add small documentation or utility stubs under `tools/` only if they help future implementation or validation
- keep them scoped to snapshot normalization / hash verification

## Acceptance criteria

- `internal/state/README.md` is sufficient for a coder to implement snapshot assembly without rereading the full tech plan
- fixtures clearly cover valid and invalid paths
- no contract drift from `spec/snapshot-contract.md`
- no edits are made outside `internal/state/`

## Forbidden changes

- no edits to shared `api/` or `spec/`
- no scheduler/evidence logic
- no unverifiable hash rules that conflict with canonical JSON guidance

## Required handoff

Fill `internal/state/HANDOFF.md` with summary, files, fixture list, validation, and unresolved contract questions.
