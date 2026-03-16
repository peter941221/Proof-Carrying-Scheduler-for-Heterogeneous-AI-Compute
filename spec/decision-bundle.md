# DecisionBundle Contract

## Purpose

`DecisionBundle` is the self-describing evidence object for one scheduling decision.

## Required invariants

- one `decision_id` identifies exactly one bundle
- `snapshot_ref.snapshot_hash` binds the decision to one snapshot
- `snapshot_ref.snapshot_version` must match the out-of-band snapshot payload `version`
- `tasks` and `assignments` are present together for independent replay
- if any `constraint_evals[].subject_id` references a task, candidate, or assignment ID, that object must be present in the bundle
- `bundle_hash` is computed from the canonical JSON payload before signing
- `prev_bundle_hash` links the bundle into an append-only chain when a previous bundle exists (omit the field when no predecessor exists)
- `fallback.code` must be present; use `FALLBACK_CODE_NONE` when no fallback occurred
- `signer_key_id` must identify the signing key or policy used to produce `signature`

## Minimum fields by certificate level

### Feasible

- `decision_id`
- `certificate_level`
- `decision_status`
- `snapshot_ref`
- `tasks`
- `assignments`
- `constraint_evals`
- `fallback`
- `bundle_hash`
- `signer_key_id`
- `signature`
- `created_at`

### Bounded

Includes feasible fields plus:

- `bound`

### Conflict

Includes feasible fields plus:

- failed `constraint_evals`
- `fallback.code != FALLBACK_CODE_NONE` if the scheduler degraded or exited early

### Counterfactual

Includes bounded or feasible fields plus:

- `counterfactuals`

### Shadow

Includes feasible fields and comparative objective terms needed by replay or shadow analysis.

## Decision status semantics

- `DECISION_STATUS_FEASIBLE`: all declared hard constraints are satisfied for the chosen assignments.
- `DECISION_STATUS_PARTIAL`: the scheduler produced a degraded decision (e.g., partial placement or relaxed objective) and records the reason in `fallback` plus any failing constraint evaluations.
- `DECISION_STATUS_INFEASIBLE`: no feasible placement was found; `assignments` may be empty and `constraint_evals` must expose the infeasibility evidence.

## Verification contract

The independent verifier must be able to:

- validate structure and field presence
- recompute canonical serialization and `bundle_hash`
- verify the signature
- re-evaluate declared constraints from bundle contents plus the referenced snapshot
- report any mismatch as a structured verification issue
