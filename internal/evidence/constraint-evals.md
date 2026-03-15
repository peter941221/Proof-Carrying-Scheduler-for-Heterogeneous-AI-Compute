# ConstraintEval Coverage and Conventions (v0)

This document standardizes how `ConstraintEval` is emitted so the verifier can replay checks and
produce stable issue codes.

References:

- `api/proto/pcs/v1/scheduler.proto` (`ConstraintEval`, `ConstraintKind`, `RelOp`)
- `spec/decision-bundle.md` (subject references)
- `spec/verification-report.md` (issue families)

## Minimum coverage for FEASIBLE decisions

For `certificate_level = CERTIFICATE_LEVEL_FEASIBLE`, evidence must include constraint witnesses that
justify why the emitted assignments are feasible. Week-1 minimum set:

1. **Resource capacity**
   - GPU count and GPU memory where relevant
   - CPU cores and memory (even for GPU jobs; helps avoid “hidden” constraints)
2. **Compatibility**
   - accelerator type match (e.g., `A100`)
   - spot eligibility for inference
3. **Placement (when requested by spec)**
   - same-zone / same-fault-domain for training
   - multi-zone / min-fault-domains for inference
4. **Budget / carbon caps** (when provided)

If a constraint category is not modeled yet, do not silently omit it:

- either mark decision as degraded and emit a fallback (`FALLBACK_CODE_DEGRADED_SIGNAL`)
- or fail/decline to claim feasibility and set status accordingly

## `constraint_id` naming

Use stable, machine-readable IDs:

- `policy.candidate_budget` (if candidate enumeration was truncated or capped)
- `resource.cpu_cores`
- `resource.memory_gib`
- `resource.gpu_count`
- `resource.gpu_memory_gib`
- `compat.accelerator_type`
- `compat.spot_allowed`
- `place.same_zone`
- `place.same_fault_domain`
- `place.multi_zone`
- `place.min_fault_domains`
- `budget.usd_per_hour`
- `carbon.gco2_per_hour`

These strings are referenced by `FallbackReason.related_constraints` and by verifier issue reports.

## Field conventions

- `subject_id`
  - prefer `assignment_id` for assignment-level constraints
  - use `task_id` for task-wide constraints
  - use a stable system-wide ID (e.g., `system`) only when truly global
- `expression`
  - human-readable, but must match the semantics the verifier implements
  - keep it short; do not embed unbounded logs
- `lhs`, `op`, `rhs`, `slack`
  - must be numerically consistent:
    - for `REL_OP_LE`: `slack = rhs - lhs`
    - for `REL_OP_GE`: `slack = lhs - rhs`
    - for `REL_OP_EQ`: `slack = -abs(lhs - rhs)`
- `satisfied`
  - true iff the operator relation holds, given any tolerance policy (week-1: exact for integers, small epsilon for floats)
- `related_ids`
  - include the minimal set needed for debugging and replay:
    - `task_id`
    - `assignment_id`
    - involved `node_id`s

## Conflict and infeasible paths

When `decision_status` is partial or infeasible:

- include at least one failing `ConstraintEval` per failed task/candidate to serve as the conflict witness
- for “no candidate” cases, emit:
  - a compatibility failure if the task cannot match any node type
  - else a resource/placement failure showing the tightest blocker found

### Unassigned task witness (week-1 minimum)

If a task is unassigned in a `DECISION_STATUS_PARTIAL` bundle, evidence must include:

- at least one failing `ConstraintEval` with:
  - `subject_id = task_id`
  - `related_ids` including `task_id` and (if applicable) the best-effort candidate/node IDs that were
    closest to feasibility
  - `constraint_id` chosen from the stable list above

If candidate enumeration was truncated (per scheduler config), evidence must additionally emit a
failing `policy.candidate_budget` `ConstraintEval` (typically `subject_id = task_id`) and set
`fallback.code = FALLBACK_CODE_SOLVER_BUDGET_EXCEEDED` so the verifier can distinguish “infeasible”
from “not fully explored”.

### Fully infeasible batch witness (week-1 minimum)

If `decision_status = DECISION_STATUS_INFEASIBLE`, evidence must include at least one failing witness
for every input task, even when no assignments are emitted.

Week-1 default conventions:

- use `subject_id = task_id` for task-level infeasibility witnesses
- include the tightest discovered blocker in `constraint_id`
- include any examined `candidate_id` or `node_id` values in `related_ids` when they help explain why
  no assignment could be produced
- pair the witnesses with `fallback.code = FALLBACK_CODE_INFEASIBLE` unless a more specific fallback
  reason such as timeout, degraded signal, or solver-budget truncation better explains the outcome
