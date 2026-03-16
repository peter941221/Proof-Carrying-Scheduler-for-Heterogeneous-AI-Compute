# Scheduler Decision Flow (v0)

This document is the implementation-facing reference for the scheduler hot path in `internal/scheduler/`.
It must remain consistent with:

- `api/proto/pcs/v1/scheduler.proto`
- `spec/decision-bundle.md`
- `spec/snapshot-contract.md`

## Goal

Given a deterministic snapshot and a batch of tasks, produce:

- a set of `Candidate` records that can be replayed by the verifier
- a chosen `Assignment` set (possibly partial)
- enough context for `internal/evidence/` to emit a `DecisionBundle`

The scheduler must not rely on hidden, non-exported state to justify decisions; if a factor matters, it
must be reflected in the bundle fields or in the referenced snapshot payload hash boundary.

## Definitions (week-1)

- **Task demand semantics:** `Task.demand` is *per-replica / per-gang-member* demand. Total demand for a
  multi-node placement is `len(node_ids) * Task.demand` (plus any policy-defined overhead, keyed by
  `SnapshotRef.policy_hash`).
- **Replica semantics (inference week-1):** one replica maps to one node. Therefore `assigned_replicas
  = len(Assignment.node_ids)` and `Candidate.node_ids` length implies replica count.
- **Gang semantics (training week-1):** `len(node_ids) == TrainingSpec.gang_size`.

## Stage order (deterministic)

1. **Input validation**
   - reject unknown references (node IDs, clusters, fault domains) before scoring
   - reject malformed or missing required task specs
2. **Candidate generation**
   - for each task, generate candidates that satisfy hard compatibility constraints
   - preserve deterministic ordering for reproducibility (see “Deterministic ordering”)
   - candidate enumeration may be capped by solver config (`SnapshotRef.solver_config_hash`) using
     deterministic truncation; if capping occurs, the scheduler must classify fallback as
     `FALLBACK_CODE_SOLVER_BUDGET_EXCEEDED` and provide enough context for evidence to emit a
     `policy.candidate_budget` witness.
3. **Fast-path feasibility checks**
   - for each candidate, evaluate hard constraints and build `ConstraintEval` witnesses
4. **Scoring**
   - compute objective terms in a stable order (see `scoring.md`)
5. **Selection**
   - choose winning assignments (one per task or partial), record rejected candidates when needed
6. **Fallback classification**
   - if the scheduler degraded, exited early, or emitted partial assignments, classify with `FallbackCode`
7. **Hand off to evidence**
   - produce a “decision context” for `internal/evidence/` with all fields needed for bundle construction

## Replayable IDs (deterministic)

To keep replay and referential integrity stable across runs, IDs must be deterministic functions of
bundle-visible content (no random UUIDs).

- `Candidate.candidate_id`:
  - format: `cand/{task_id}/{node_id_1}+{node_id_2}+...`
  - where `node_ids` is already in deterministic order
- `Assignment.assignment_id`:
  - format: `asgn/{task_id}/{node_id_1}+{node_id_2}+...`

If IDs would exceed an implementation limit, the implementation may replace the `{...}` suffix with
a deterministic hash (e.g., hex SHA-256 of the suffix) **as long as** the mapping rule is stable and
documented in code.

## Candidate field derivation (must be replayable)

For every emitted `Candidate`:

- `Candidate.node_ids`: deterministic order (see “Deterministic ordering” below).
- `Candidate.cluster_id`, `Candidate.region`:
  - derived from the selected nodes.
  - **must be consistent across nodes**; if nodes span multiple clusters/regions, the candidate is
    invalid for week-1 and must not be emitted.
- `expected_cost_usd_per_hour`:
  - computed from snapshot node pricing (`Node.gpu_hour_cost_usd`, `Node.cpu_hour_cost_usd`) and
    per-node resource consumption implied by `Task.demand` (plus policy-defined overhead).
  - rule (week-1): for each selected node, cost per hour is:
    - `Task.demand.gpu_count * Node.gpu_hour_cost_usd + Task.demand.cpu_cores * Node.cpu_hour_cost_usd`
    - (if demand uses fractional GPU/CPU, treat as proportional)
  - candidate cost is the sum across `Candidate.node_ids`.
- `expected_carbon_gco2_per_hour`:
  - computed from snapshot node carbon intensity (`Node.carbon_gco2_per_kwh`) and policy-defined power
    model keyed by `SnapshotRef.policy_hash`.
  - week-1 minimal deterministic model (if no better model is present): `power_watts =
    Task.demand.power_watts` and `gco2_per_hour = (power_watts / 1000) * carbon_gco2_per_kwh`.
  - candidate carbon is the sum across `Candidate.node_ids`.
- `expected_p95_latency_ms`:
  - computed from `SnapshotMetadata.network_edges` only.
  - week-1 rule: for a multi-node placement, use the maximum pairwise `p95_latency_ms` across the
    selected nodes where an edge exists in either direction.
  - if any required edge is missing **and** latency is a modeled term for this decision, the scheduler
    must mark the decision as degraded (`FALLBACK_CODE_DEGRADED_SIGNAL`) rather than silently emitting
    `0`.
- `interruption_risk`:
  - week-1 rule: `1 - Π(1 - Node.interruption_prob)` across selected nodes.
- `topology_score`:
  - week-1 rule: `1.0` if all nodes share the same `fault_domain`, else `0.0` (placeholder but
    deterministic). If `TrainingSpec.topology_sensitive = true` and topology scoring is not modeled
    beyond this placeholder, emit degraded fallback.
- `objective_terms`:
  - must follow ordering and normalization rules in `scoring.md`.
  - *every* modeled term that can affect selection must appear here.

## Training-path candidate flow (week-1 scope)

For `Task.kind = TASK_KIND_TRAINING`:

- **Gang sizing**
  - treat `TrainingSpec.gang_size` as the required `node_ids` count for a candidate
  - emit candidates only when a full gang can be assembled
- **Accelerator compatibility**
  - candidate nodes must match `TrainingSpec.allowed_accelerators` when provided
  - set `ResourceVector.accelerator_type` in snapshot nodes so this can be replayed
- **Placement constraints**
  - if `requires_same_zone = true`, all nodes must share one `Node.zone`
  - if `requires_same_fault_domain = true`, all nodes must share `Node.fault_domain`
- **Budget / carbon**
  - if present, treat `max_budget_usd_per_hour` and `max_carbon_gco2_per_hour` as hard constraints

Output requirements:

- `Candidate.node_ids` must list the gang nodes in deterministic order (see ordering rules)
- `Candidate.expected_cost_usd_per_hour` / `expected_carbon_gco2_per_hour` must be derived from bundle/snapshot fields only

## Inference-path candidate flow (week-1 scope)

For `Task.kind = TASK_KIND_INFERENCE`:

- **Replica sizing**
  - candidates represent a specific replica count and placement set
  - record `Assignment.assigned_replicas` and `assigned_qps`
- **Spot policy**
  - if `InferenceSpec.can_use_spot = false`, exclude `Node.spot = true`
- **Multi-zone / fault domain**
  - if `requires_multi_zone = true`, candidate must span at least 2 zones
  - if `min_fault_domains > 0`, candidate must span that many distinct `Node.fault_domain`
- **SLO-aware filtering**
  - for week-1, treat `p95_slo_ms` as a hard filter against `Candidate.expected_p95_latency_ms` where computable

Output requirements:

- `Candidate.expected_p95_latency_ms` must be computed from snapshot network edges. If latency is
  modeled but cannot be computed due to missing edges, emit degraded fallback (do not emit `0`).

## Deterministic ordering

To make bundle replay stable:

- Sort tasks by `(priority desc, urgency desc, task_id asc)`.
- For candidate generation:
  - build a stable list of eligible nodes sorted by `node_id asc`
  - enumerate candidate node-sets in lexicographic order of the `node_ids` tuple
- Preserve the same task order when emitting `tasks`, and use that same task order as the primary key
  when evidence sorts `candidates` and `assignments` for canonical JSON.
- When selecting winners, break ties by:
  - total `Assignment` contribution (higher is better)
  - then tie-break contract from `scoring.md`

## Selection algorithm (week-1, deterministic greedy)

Week-1 selection is explicitly defined as a deterministic greedy pass (no hidden heuristics):

1. Sort tasks as described in “Deterministic ordering”.
2. For each task in order:
   1. Build that task's candidate list (already deterministically ordered at generation time).
   2. Re-score candidates (if needed) using `scoring.md` and sort by:
      - total contribution desc
      - then tie-break contract from `scoring.md`
   3. For each candidate in that order, run **global** feasibility checks against remaining
      `Node.capacity_free` (subtracting the demands of already-chosen assignments in this same call).
   4. Choose the first globally feasible candidate and emit an `Assignment` derived from it:
      - copy `node_ids`, `cluster_id`, `region`, all expected_* fields, `interruption_risk`,
        and `objective_terms`
      - set:
        - for inference: `assigned_replicas = len(node_ids)`
        - for training: `assigned_replicas = 0` (unused), `assigned_qps = 0`
        - for inference: `assigned_qps` computed by a policy-defined throughput model keyed by
          `SnapshotRef.policy_hash`; if not modeled, emit degraded fallback
   5. If no candidate is globally feasible, the task remains unassigned for this decision.

## Decision status (must be explicit)

- `DECISION_STATUS_FEASIBLE`: every input task has exactly one emitted assignment.
- `DECISION_STATUS_PARTIAL`: at least one task has an assignment and at least one task is unassigned.
- `DECISION_STATUS_INFEASIBLE`: no tasks could be assigned (empty `assignments`), or the scheduler
  claims infeasibility for all tasks.

Evidence must receive enough context to emit failing `ConstraintEval` witnesses for unassigned tasks
(see `internal/evidence/constraint-evals.md`).

## Evidence-facing decision context

The scheduler handoff to evidence must include, in addition to the emitted bundle objects:

- the deterministic scheduler task order used for this decision
- for each task, the full enumerated candidate set that was actually considered before any selection
- for each unassigned task, the tightest failing constraint category and the best-effort blocker IDs
- whether candidate enumeration was truncated by policy/solver budget
- whether any modeled term or throughput signal was unavailable and therefore forced degradation

These fields are required so evidence can choose the right `certificate_level`, emit replayable
conflict witnesses, and distinguish true infeasibility from partial exploration.

## Fallback trigger surface

Fallback is emitted when any of the following occurs:

- timeout of a solver stage (`FALLBACK_CODE_TIMEOUT`)
- solver budget exceeded (`FALLBACK_CODE_SOLVER_BUDGET_EXCEEDED`)
- no feasible candidate exists for at least one task (`FALLBACK_CODE_NO_CANDIDATE`)
- a policy override changes the selected assignment (`FALLBACK_CODE_POLICY_OVERRIDE`)
- hard infeasibility is detected (`FALLBACK_CODE_INFEASIBLE`)
- degraded inputs (missing edges, stale metrics) force a simplified decision (`FALLBACK_CODE_DEGRADED_SIGNAL`)

If none of the fallback triggers fire, evidence should omit the `fallback` field entirely; the
scheduler should not request an emitted `FallbackReason` with `FALLBACK_CODE_NONE` for a normal
feasible path.

The scheduler must provide enough structured context so evidence can populate:

- `FallbackReason.code`
- `FallbackReason.message`
- `FallbackReason.related_constraints` (constraint IDs, not free-form text)
