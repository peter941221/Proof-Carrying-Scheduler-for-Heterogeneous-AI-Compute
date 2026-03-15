# API Module

This directory is the shared wire-contract layer for the scheduler and verifier.

## Owns

- protobuf packages
- RPC request / response shapes
- enum vocabularies used across modules

## Source of truth

- `proto/pcs/v1/scheduler.proto`
- `../spec/contract-packet.md`
- `../spec/decision-bundle.md`
- `../spec/verification-report.md`

## For module agents

If you open the `module/api-spec` worktree, treat this directory plus `../spec/` as your local project root.

## Must preserve

- stable field names once other modules depend on them
- certificate and verification terminology alignment
- compatibility with the shared claim vocabulary

## First focus

- tighten field naming and comments
- keep issue codes and claim statuses consistent with the verifier contract
- avoid leaking implementation-only concepts into the public API
