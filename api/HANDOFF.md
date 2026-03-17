# API Handoff

## Status

- `ready`

## Summary

- Tightened verifier-facing protobuf comments around fallback semantics, counterfactual interpretation, bundle hashing, signer-policy lookup, and verification result booleans.
- Kept the wire surface frozen: no field renames, enum changes, or new fields.
- Revalidated the frozen protobuf surface while the spec completed the contract changelog template deliverable.

## Files changed

- `api/proto/pcs/v1/scheduler.proto`

## Validation run

- command: `protoc -I api/proto -I C:\Users\peter\anaconda3\Library\include --descriptor_set_out=$env:TEMP\pcs_api.desc --include_imports api/proto/pcs/v1/scheduler.proto`
- result: pass

## Contract impact

- Comment-only clarification; no wire-format or field-shape changes.
- Clarified:
  - `FallbackReason.message` and `related_constraints` intent
  - counterfactual feasibility/objective comparability boundaries
  - `assignments` emptiness only for infeasible decisions
  - `bundle_hash` chain coverage when `prev_bundle_hash` is present
  - `signer_key_id` as the lookup key for signature procedure
  - `VerifyResponse` booleans when checks are skipped under contract rules
- This round made no additional API/protobuf edits.

## Risks / follow-ups

- open issues:
- commander decision needed:
  - Decide whether `signature` must always cover `bundle_hash` only, or whether signer-policy-defined canonical-byte signing should remain allowed.
