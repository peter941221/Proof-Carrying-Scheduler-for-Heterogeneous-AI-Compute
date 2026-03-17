# Spec Handoff

## Status

- `ready`

## Summary

- Added a commander-facing contract changelog template so future frozen-surface changes can be recorded consistently.
- Linked the contract packet to that template and kept the contract frozen otherwise.

## Files changed

- `spec/contract-changelog-template.md`
- `spec/contract-packet.md`

## Validation run

- command: `python -m json.tool spec/examples/decision-bundle.minimal.json > $null`
- result: pass
- command: `protoc -I api/proto -I C:\Users\peter\anaconda3\Library\include --descriptor_set_out=$env:TEMP\pcs_api.desc --include_imports api/proto/pcs/v1/scheduler.proto`
- result: pass

## Resolved ambiguities

- The contract packet now points to a single place for recording future commander-reviewed contract changes.
- The changelog template now requires change scope, wire impact, downstream impact, validation, and migration notes in one consistent entry shape.

## Risks / follow-ups

- open issues:
- commander decision needed:
  - Confirm whether the signer policy may choose between signing `bundle_hash` and signing canonical bytes, or whether the contract should collapse to a single required signature mode.
