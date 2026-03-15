# DecisionBundle Contract

## Purpose

`DecisionBundle` is the self-describing evidence object for one scheduling decision.

## Required invariants

- one `decision_id` identifies exactly one bundle
- `snapshot_ref.snapshot_hash` binds the decision to one snapshot
- `tasks`, `candidates`, and `assignments` are included together for independent replay
- every `constraint_eval.subject_id` points at a task, candidate, assignment, or system-wide subject present in the bundle
- `bundle_hash` is computed from the canonical JSON payload before signing
- `prev_bundle_hash` links the bundle into an append-only chain when a previous bundle exists

## Minimum fields by certificate level

### Feasible

- `decision_id`
- `certificate_level`
- `decision_status`
- `snapshot_ref`
- `tasks`
- `assignments`
- `constraint_evals`
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
- `fallback` if the scheduler degraded or exited early

### Counterfactual

Includes bounded or feasible fields plus:

- `counterfactuals`

### Shadow

Includes feasible fields and comparative objective terms needed by replay or shadow analysis.

## Verification contract

The independent verifier must be able to:

- validate structure and field presence
- recompute canonical serialization and `bundle_hash`
- verify the signature
- re-evaluate declared constraints from bundle contents plus the referenced snapshot
- report any mismatch as a structured verification issue
