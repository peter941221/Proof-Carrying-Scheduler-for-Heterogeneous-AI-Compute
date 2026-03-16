# Spec Handoff

## Status

- `ready`

## Summary

- Added explicit `ClaimCheck.artifact_refs` placeholder guidance so each initial claim family has a stable artifact channel reserved.
- Kept the contract frozen otherwise; no semantic expansion beyond the coordination-packet deliverable.

## Files changed

- `spec/claim-taxonomy.md`

## Validation run

- command: `python -m json.tool spec/examples/decision-bundle.minimal.json > $null`
- result: pass
- command: `protoc -I api/proto -I C:\Users\peter\anaconda3\Library\include --descriptor_set_out=$env:TEMP\pcs_api.desc --include_imports api/proto/pcs/v1/scheduler.proto`
- result: pass

## Resolved ambiguities

- `ClaimCheck.artifact_refs` now has a recommended stable placeholder format: `<artifact_type>:<stable_id>`.
- Every initial claim family now has at least one placeholder artifact reference channel reserved (`model`, `log`, or `report`) so downstream modules can attach artifacts consistently.
- Placeholder refs are explicitly non-evidentiary and do not justify raising `ClaimStatus`.

## Risks / follow-ups

- open issues:
- commander decision needed:
  - Confirm whether the signer policy may choose between signing `bundle_hash` and signing canonical bytes, or whether the contract should collapse to a single required signature mode.
