# Verification Issue Codes (Catalog)

This catalog defines recommended verifier issue codes under the families listed in `spec/verification-report.md`.

The verifier may add additional codes, but should preserve the family prefixes so downstream filtering remains stable.

## STRUCT.*

Structure, field presence, and referential integrity.

- `STRUCT.DECODE_FAILED` (CRITICAL): bundle cannot be decoded / parsed
- `STRUCT.MISSING_REQUIRED_FIELD` (ERROR): missing a required field for basic verification
- `STRUCT.UNSUPPORTED_CERTIFICATE_LEVEL` (ERROR|WARNING): unknown `certificate_level`
- `STRUCT.DUPLICATE_ID` (ERROR): duplicate IDs within `tasks`, `candidates`, or `assignments`
- `STRUCT.UNRELATED_RELATED_ID` (WARNING): `related_ids` cannot be validated (should be rare)
- `STRUCT.SUBJECT_NOT_FOUND` (ERROR): `constraint_eval.subject_id` cannot be resolved
- `STRUCT.EMPTY_ASSIGNMENTS` (ERROR|WARNING): empty assignment set under a status/level that implies assignments exist

## HASH.*

Canonicalization and bundle hash consistency per `spec/canonical-json.md`.

- `HASH.CANONICALIZATION_FAILED` (CRITICAL): cannot produce canonical JSON bytes
- `HASH.BUNDLE_HASH_MISSING` (ERROR): `bundle_hash` absent when required
- `HASH.BUNDLE_HASH_MISMATCH` (CRITICAL): recomputed hash does not match `bundle_hash`
- `HASH.PREV_HASH_FORMAT_INVALID` (ERROR): `prev_bundle_hash` malformed

## SIGNATURE.*

Signature validation policy.

- `SIGNATURE.MISSING_SIGNATURE` (ERROR): signature missing when `verify_signature = true`
- `SIGNATURE.MISSING_SIGNER_KEY_ID` (ERROR): `signer_key_id` missing when `verify_signature = true`
- `SIGNATURE.KEY_NOT_FOUND` (ERROR): signer key cannot be resolved
- `SIGNATURE.INVALID` (CRITICAL): signature check failed
- `SIGNATURE.SKIPPED` (INFO): signature verification intentionally disabled

## SNAPSHOT.*

Snapshot binding rules per `spec/snapshot-contract.md`.

- `SNAPSHOT.REF_MISSING` (ERROR): `snapshot_ref` missing
- `SNAPSHOT.PAYLOAD_MISSING` (ERROR|WARNING): snapshot payload not provided when needed for replay
- `SNAPSHOT.HASH_MISMATCH` (CRITICAL): snapshot payload hash != `snapshot_ref.snapshot_hash`
- `SNAPSHOT.VERSION_MISMATCH` (ERROR|WARNING): snapshot version conflicts with supported verifier rules
- `SNAPSHOT.SKIPPED` (INFO): snapshot verification intentionally disabled (only if replay not requested)

## CONSTRAINT.*

Constraint replay of `constraint_evals`.

- `CONSTRAINT.EVAL_MISSING` (ERROR): expected constraint evaluation is missing for a declared subject
- `CONSTRAINT.EVAL_FAILED` (ERROR): replayed evaluation fails against bundle + snapshot
- `CONSTRAINT.KIND_UNSUPPORTED` (ERROR|WARNING): unknown `constraint_kind`
- `CONSTRAINT.SKIPPED` (INFO): constraint verification intentionally disabled

## BOUND.*

Bound certificate checks (only meaningful when `certificate_level` implies a bound).

- `BOUND.MISSING_CERTIFICATE` (ERROR): bound required but absent
- `BOUND.MISMATCH` (ERROR): bound fields inconsistent with replayed objective or declared status
- `BOUND.SKIPPED` (INFO): bound verification intentionally disabled

## COUNTERFACTUAL.*

Counterfactual checks (only meaningful when counterfactuals are present or level requires them).

- `COUNTERFACTUAL.MISSING` (ERROR): counterfactuals required but absent
- `COUNTERFACTUAL.INVALID` (ERROR): counterfactual semantics inconsistent with bundle or snapshot
- `COUNTERFACTUAL.SKIPPED` (INFO): counterfactual verification intentionally disabled

## CLAIM.*

Issues related to claim reporting completeness and consistency.

- `CLAIM.MISSING_CHECKED_CLAIMS` (WARNING): `checked_claims` empty when at least one stage ran
- `CLAIM.UNKNOWN_CLAIM_ID` (WARNING): claim ID not in `spec/claim-taxonomy.md`
- `CLAIM.SKIPPED_STAGE` (INFO): a stage was disabled; checked-claim status must not imply success
- `CLAIM.OVERSTATED_STATUS` (ERROR): a claim is marked stronger than the stage evidence supports

## `related_ids` guidance

Populate `related_ids` with stable identifiers where possible:

- `decision_id`
- `snapshot_ref.snapshot_id`
- `task_id`, `candidate_id`, `assignment_id`
- any constraint-specific ID if modeled (e.g., `constraint_eval_id` if added later)

Order `related_ids` deterministically and keep the list short (pinpoint, do not spam).

