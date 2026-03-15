# Evidence Emission (DecisionBundle) (v0)

This document defines how `internal/evidence/` turns scheduler outputs into a replayable and
independently checkable `DecisionBundle`.

It must remain consistent with:

- `api/proto/pcs/v1/scheduler.proto` (`DecisionBundle`, `ConstraintEval`, `BoundCertificate`, `FallbackReason`)
- `spec/decision-bundle.md`
- `spec/canonical-json.md`
- `spec/verification-report.md`

## Non-negotiable invariants

- Evidence must be verifiable using only:
  - bundle fields
  - referenced snapshot payload (bound by `snapshot_ref.snapshot_hash`)
  - verifier-owned code (no trust in scheduler internals)
- `bundle_hash` is computed from canonical JSON with `bundle_hash` and `signature` omitted.
- `prev_bundle_hash` (when present) is included in the next hash input.

## Construction steps (deterministic)

1. **Assemble payload**
   - populate `DecisionBundle` fields from scheduler decision context
   - ensure minimum fields for the selected `certificate_level` per `spec/decision-bundle.md`
   - **sort repeated fields** into a deterministic semantic order (see “Deterministic ordering”)
2. **Structure checks**
   - ensure every `constraint_eval.subject_id` references an object present in the bundle, except the
     reserved global subject ID `system`
   - ensure all `candidate_id` / `assignment_id` strings match the deterministic rules in
     `internal/scheduler/decision-flow.md`
3. **Canonicalize + hash**
   - apply `spec/canonical-json.md` rules
   - compute `bundle_hash` over canonical JSON bytes with `bundle_hash` + `signature` omitted
4. **Sign**
   - set `signer_key_id`
   - produce `signature` over hash bytes (or canonical bytes as policy dictates)
5. **Finalize**
   - set `created_at`
   - optionally set `prev_bundle_hash` if chaining is enabled

## Deterministic ordering (required because JSON arrays preserve order)

Because `spec/canonical-json.md` preserves array order, evidence must impose stable ordering on every
repeated field in `DecisionBundle`:

- `tasks`: use the scheduler task order (see `internal/scheduler/decision-flow.md`), i.e.
  `(priority desc, urgency desc, task_id asc)`.
- `candidates`: stable sort by `(task_id asc, candidate_id asc)`. (Within a task, `candidate_id`
  already encodes deterministic node ordering.)
- `assignments`: stable sort by `(task_id asc, assignment_id asc)`.
- `constraint_evals`: stable sort by `(subject_id asc, constraint_id asc, satisfied desc)`.
- `counterfactuals` (if present): stable sort by `(counterfactual_id asc)`.

## Certificate level mapping (week-1)

### FEASIBLE

Use `certificate_level = CERTIFICATE_LEVEL_FEASIBLE` when the bundle is making only a feasibility
claim for the emitted assignments.

Always include:

- `decision_id`
- `certificate_level = CERTIFICATE_LEVEL_FEASIBLE`
- `decision_status` (`DECISION_STATUS_FEASIBLE`, `DECISION_STATUS_PARTIAL`, or `DECISION_STATUS_INFEASIBLE` when the implementation is not making a stronger conflict certificate claim)
- `snapshot_ref`
- `tasks`
- `candidates` (required for independent replay per `spec/decision-bundle.md`)
- `assignments`
- `constraint_evals` (see `constraint-evals.md`)
- `bundle_hash`, `signer_key_id`, `signature`, `created_at`

Week-1 default: partial decisions stay at `CERTIFICATE_LEVEL_FEASIBLE` unless the implementation is
explicitly claiming a conflict certificate instead.

### BOUNDED

Use `certificate_level = CERTIFICATE_LEVEL_BOUNDED` only when the bundle carries a valid solver bound
for the emitted incumbent assignments.

Include FEASIBLE fields plus:

- `bound`

### CONFLICT

Use `certificate_level = CERTIFICATE_LEVEL_CONFLICT` only when the implementation is explicitly
claiming a conflict/infeasibility witness rather than only a feasible or partial assignment result.

Include FEASIBLE fields plus:

- failed `constraint_evals` showing the conflict witness
- `fallback` when the scheduler degraded or exited early

Week-1 default: fully unassigned outputs with `DECISION_STATUS_INFEASIBLE` may be emitted as either
`CERTIFICATE_LEVEL_FEASIBLE` plus fallback/conflict witnesses, or `CERTIFICATE_LEVEL_CONFLICT` if the
implementation is making that stronger claim. The chosen policy must be deterministic within one
implementation.

### COUNTERFACTUAL / SHADOW

Week-1: allowed but optional.

- `CERTIFICATE_LEVEL_COUNTERFACTUAL` requires the base feasible/bounded payload plus populated
  `counterfactuals`.
- `CERTIFICATE_LEVEL_SHADOW` is reserved for comparative or shadow-analysis output and must still keep
  the feasible-path fields replayable.
- If counterfactual or shadow analysis is not intentionally produced, omit both the elevated
  certificate levels and the `counterfactuals` field entirely.

If emitted, ensure counterfactual payloads reference only snapshot + bundle data.

## Fallback payload behavior

Emit `fallback` when:

- the decision is partial (`DECISION_STATUS_PARTIAL`)
- the scheduler selected a degraded policy path
- a timeout/budget cap prevented full search
- the scheduler declares infeasibility (`DECISION_STATUS_INFEASIBLE`)

Rules:

- `FallbackReason.code` must be one of the `FallbackCode` enum values (no free-form categories).
- `FallbackReason.related_constraints` must contain stable `constraint_id` strings (not node/task IDs).
- `FallbackReason.message` may be human-readable but must not carry verifier-required structure.
- If no fallback condition fired, omit the `fallback` field entirely; do not emit a placeholder
  `FallbackReason` with `FALLBACK_CODE_NONE`.

## Bound / optional certificates (week-1)

- If `certificate_level = CERTIFICATE_LEVEL_BOUNDED`, `bound` must be present and internally
  consistent (`best_bound`, `incumbent_objective`, `relative_gap`, `timed_out`).
- If no valid bound data exists, evidence must **not** claim `BOUNDED`; keep `certificate_level =
  FEASIBLE` (or `CONFLICT` as appropriate) and omit `bound`.
- If the scheduler ran a solver that produced both an incumbent assignment set and a bound but then
  took a fallback path (timeout/budget), evidence should:
  - keep the chosen assignments,
  - set `fallback` accordingly,
  - and include `bound` **only** if still claiming `BOUNDED` (week-1 default: do not).

## Verifier-facing completeness check (week-1)

Before signing, evidence must confirm:

- every `Assignment.task_id` exists in `tasks`
- every `Candidate.task_id` (if present) exists in `tasks`
- every `ConstraintEval.related_ids` references known IDs (task_id / node_id / assignment_id / candidate_id)
- `SnapshotRef.snapshot_hash` is present and non-empty
