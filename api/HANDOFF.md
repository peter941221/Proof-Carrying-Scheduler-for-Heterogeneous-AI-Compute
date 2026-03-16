# API Handoff

## Status

- `ready`

## Summary

- Tightened verifier-facing protobuf comments for hash/signature-sensitive fields and evidence semantics.
- Added explicit contract comments for `DecisionBundle`, `SnapshotRef`, `VerificationIssue`, `ClaimCheck`, and verification toggles.

## Files changed

- `api/proto/pcs/v1/scheduler.proto`

## Validation run

- command: `protoc -I api/proto -I C:\Users\peter\anaconda3\Library\include --descriptor_set_out=%TEMP%\pcs_api.desc --include_imports api/proto/pcs/v1/scheduler.proto`
- result: pass

## Contract impact

- Added contract comments (no field/enum renames, no wire changes).
- Clarified:
  - snapshot version binding (`SnapshotRef.snapshot_version` must match out-of-band snapshot payload `version`)
  - bundle hash inputs (canonical JSON; omit `bundle_hash` + `signature`)
  - append-only chaining expectations for `prev_bundle_hash`
  - verifier issue code intent + claim status meaning (aligned to `spec/`)

## Risks / follow-ups

- open issues:
- commander decision needed:
  - Decide whether to make RFC 8785 / JCS number formatting a MUST (currently only recommended in `spec/canonical-json.md`).
