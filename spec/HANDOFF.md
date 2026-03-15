# Spec Handoff

## Status

- `ready`

## Summary

- Closed key semantic gaps across decision bundle / snapshot / verification / claims docs so modules can implement without guessing.
- Aligned terminology and added missing invariants (snapshot version binding, fallback presence, decision status semantics, claim status vocabulary).
- Tightened canonicalization guidance (recommended RFC 8785 / JCS number formatting; UTC `Z` timestamps).

## Files changed

- `spec/canonical-json.md`
- `spec/claim-taxonomy.md`
- `spec/contract-packet.md`
- `spec/decision-bundle.md`
- `spec/snapshot-contract.md`
- `spec/verification-report.md`
- `spec/examples/decision-bundle.minimal.json`

## Validation run

- command: `python -m json.tool spec/examples/decision-bundle.minimal.json`
- result: pass

## Resolved ambiguities

- `SnapshotRef.snapshot_version` vs snapshot payload `version` binding is now explicitly required and reflected in the minimal example.
- `VerifyRequest` skip/strict behavior is spelled out normatively in `verification-report.md`.
- Claim status wording is aligned to protobuf `ClaimStatus` values (no free-form statuses).

## Risks / follow-ups

- open issues:
- commander decision needed:
  - Confirm whether hash string representation should be fully specified (e.g. require `sha256:<hex>`), or remain an opaque string with a recommended format.
