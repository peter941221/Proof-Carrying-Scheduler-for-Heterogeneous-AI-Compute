# Scheduler Handoff

## Status

- ready

## Summary

- Locked week-1 deterministic scheduler hot-path semantics: stage order, replayable IDs, candidate/assignment field derivation, explicit greedy selection algorithm, and explicit decision_status mapping.
- Tightened evidence handoff expectations by requiring scheduler-owned decision-context fields for full candidate coverage, blocker reporting, truncation reporting, and degraded-signal reporting.
- Clarified normal feasible-path fallback omission: implementations should omit `fallback` entirely instead of emitting `FALLBACK_CODE_NONE`.
- Locked scoring normalization to be policy-hash based (replayable by verifier) and removed “per-call min/max” dependence that couldn’t be recomputed from bundle alone.

## Files changed

- `internal/scheduler/decision-flow.md`
- `internal/scheduler/scoring.md`

## Decision-path coverage

- training path: `TASK_KIND_TRAINING` gang sizing (`gang_size`), accelerator compatibility, same-zone / same-fault-domain placement, budget & carbon hard caps.
- inference path: `TASK_KIND_INFERENCE` replica semantics (1 replica = 1 node), spot eligibility, multi-zone / min fault domains, latency term rules (missing edges => degraded fallback).
- fallback path: explicit triggers incl. timeout/budget/no-candidate/policy override/infeasible/degraded signals; added explicit rule for candidate enumeration caps => `FALLBACK_CODE_SOLVER_BUDGET_EXCEEDED` (+ `policy.candidate_budget` witness requirement for evidence).
- evidence handoff path: requires deterministic task order, full considered candidate set, unassigned-task blocker metadata, truncation flags, and degraded-signal flags so evidence can choose replay-safe certificate/fallback behavior.

## Validation run

- command: `rg -n "Evidence-facing decision context|FALLBACK_CODE_NONE|DECISION_STATUS_INFEASIBLE" internal/scheduler internal/evidence`
- result: pass
- command: `git diff -- internal/scheduler/decision-flow.md`
- result: pass

## Risks / follow-ups

- open issues:
  - Throughput model for `Assignment.assigned_qps` is specified as policy-hash keyed, but the concrete policy schema/location is out-of-band and must be defined/implemented by the state/policy module.
  - Carbon/power model defaults are deterministic but likely too naive; refine under `policy_hash` without changing replay contract.
- commander decision needed:
  - Confirm whether week-1 should *always* include all enumerated candidates in the bundle (recommended for replay) vs top-K; evidence docs currently assume candidates are required for replay.
