# Verification Stages (Order and Guarantees)

This document defines the verifier's stage order and how each stage affects the `VerifyResponse` flags and issues.

## Stage order (normative)

```text
S0  Parse + Basic decoding
S1  Structural validation (required fields, referential integrity)
S2  Canonical JSON + bundle_hash recomputation
S3  Signature verification (optional by request)
S4  Snapshot binding verification (hash/metadata match)
S5  Constraint replay (optional by request)
S6  Bound verification (optional by request)
S7  Counterfactual verification (optional by request)
S8  Checked-claims synthesis (always; based on which stages ran)
```

## Stage gating

The verifier must always execute S0-S2.

Additional stages are controlled by `VerifyRequest`:

- `verify_signature`: enables S3
- `verify_constraints`: enables S5
- `verify_bound`: enables S6
- `verify_counterfactuals`: enables S7
- `strict_mode`: tightens validation (see below)

## Failure handling rules

### Hard-stop failures

These failures must stop later stages that depend on the missing prerequisite and produce a `CRITICAL` issue:

- S0 fails (bundle cannot be decoded) -> stop all later stages
- S1 fails in a way that makes IDs / references unusable -> stop S5-S7
- S2 fails (canonicalization or hash recomputation) -> stop S3 and any stage that relies on stable identity via `bundle_hash`
- S4 fails (snapshot binding cannot be established) -> stop S5-S7 when snapshot is required for replay

### Soft failures

If a stage is intentionally skipped due to `VerifyRequest` flags, the verifier must:

- set the corresponding boolean flag to `false`
- emit a `CLAIM.SKIPPED_STAGE` or stage-specific `INFO`/`WARNING` issue so downstream consumers do not mistake omission for success

If a stage is `blocked` by an earlier hard-stop, the verifier must:

- set the corresponding boolean flag to `false` if the stage owns one
- emit the stage-specific failure cause from the earlier stage first
- emit at most one stage-local `INFO`/`WARNING` issue only when needed to explain why a claim was downgraded or omitted
- avoid emitting `*.SKIPPED` for blocked stages; `blocked` is not an operator choice

## `VerifyResponse` flag semantics

`VerifyResponse.valid` is the aggregate truth of the verification attempt under the given `VerifyRequest` policy.

Set `valid` deterministically from final stage outcomes as follows:

- `valid = false` if any executed mandatory stage (`S0-S4`) ends `failed`
- `valid = false` if any enabled optional stage (`S5-S7`) ends `failed`
- `valid = false` if claim synthesis (`S8`) detects an overstated claim status and emits `CLAIM.OVERSTATED_STATUS`
- `valid = true` otherwise, including when optional stages are `skipped` because the request disabled them

Blocked stages do not independently make the response valid; they inherit the invalidating earlier hard-stop that caused the block.

Per-stage flags remain stage-local evidence markers:

- `signature_valid = true` implies S3 ran and the signature check succeeded
- `constraints_valid = true` implies S5 ran and all declared checks succeeded
- `bound_valid = true` implies S6 ran and bound semantics match the bundle
- `counterfactuals_valid = true` implies S7 ran and counterfactual semantics match the bundle

If a stage is disabled by request, the corresponding flag must be `false`.

## Stage outcome table (implementation guide)

Use the following table to derive flags and claim-synthesis inputs deterministically.

```text
Stage  Outcome   Flag effect                  Claim synthesis input
S0     passed    none                         enables S1-S8
S0     failed    valid = false               mark S1-S8 as blocked

S1     passed    none                         enables dependent stages
S1     failed    valid = false               mark unusable dependent stages blocked

S2     passed    none                         may support VERIFY.BUNDLE_SOUNDNESS
S2     failed    valid = false               mark S3-S8 bundle-dependent outputs blocked

S3     passed    signature_valid = true       may support future signature-related claims
S3     failed    signature_valid = false      no claim success from S3
S3     skipped   signature_valid = false      emit skipped-stage claim input
S3     blocked   signature_valid = false      emit blocked-stage claim input

S4     passed    none                         may support SAFETY.SNAPSHOT_CONSISTENCY
S4     failed    valid = false               mark snapshot-backed S5-S7 blocked
S4     skipped   none                         only allowed when no enabled stage requires snapshot proof
S4     blocked   none                         weaker than skipped; no success implied

S5     passed    constraints_valid = true     may support VERIFY.CONFLICT_SOUNDNESS
S5     failed    constraints_valid = false    no claim success from S5
S5     skipped   constraints_valid = false    emit skipped-stage claim input
S5     blocked   constraints_valid = false    emit blocked-stage claim input

S6     passed    bound_valid = true           may support BOUND.RELATIVE_GAP_REPORTING
S6     failed    bound_valid = false          no claim success from S6
S6     skipped   bound_valid = false          emit skipped-stage claim input
S6     blocked   bound_valid = false          emit blocked-stage claim input

S7     passed    counterfactuals_valid = true reserved for future claim mapping
S7     failed    counterfactuals_valid = false no claim success from S7
S7     skipped   counterfactuals_valid = false emit skipped-stage claim input
S7     blocked   counterfactuals_valid = false emit blocked-stage claim input

S8     always    no direct flag change        emit final `checked_claims`
```

## Strict mode (recommended default)

When `strict_mode = true`, treat the following as `ERROR` instead of `WARNING`:

- unknown or unsupported `certificate_level`
- missing fields that are required for the claimed `certificate_level`
- any `constraint_eval.subject_id` that cannot be resolved to a subject present in the bundle
- ambiguous identifiers (duplicate IDs in `tasks`, `candidates`, `assignments`)

## Output ordering

For stable diffs and deterministic verifier behavior, `issues` should be sorted by:

1. stage order (S0 -> S8)
2. severity (CRITICAL -> INFO)
3. `code` lexicographically
4. stable `related_ids` ordering
