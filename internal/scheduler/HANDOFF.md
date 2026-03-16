# Scheduler Handoff

## Status

- ready

## Summary

- Locked week-1 deterministic scheduler hot-path semantics: stage order, replayable IDs, candidate/assignment field derivation, explicit greedy selection algorithm, and explicit decision_status mapping.
- Tightened evidence handoff expectations by requiring scheduler-owned decision-context fields for full candidate coverage, blocker reporting, truncation reporting, and degraded-signal reporting.
- Clarified normal feasible-path fallback omission: implementations should omit `fallback` entirely instead of emitting `FALLBACK_CODE_NONE`.
- Tightened tie-breaking and emission ordering by routing selection ties through `scoring.md` and requiring evidence to preserve scheduler task order when sorting `candidates` and `assignments`.

## Files changed

- `internal/scheduler/decision-flow.md`
- `internal/scheduler/scoring.md`

## Decision-path coverage

- training path: `TASK_KIND_TRAINING` gang sizing (`gang_size`), accelerator compatibility, same-zone / same-fault-domain placement, budget & carbon hard caps.
- inference path: `TASK_KIND_INFERENCE` replica semantics (1 replica = 1 node), spot eligibility, multi-zone / min fault domains, latency term rules (missing edges => degraded fallback).
- scoring/tie path: objective term order, policy-hash keyed normalization, policy-defined epsilon, latency-aware tie-breaks, and final lexicographic ID fallback are now explicit.
- evidence handoff path: requires deterministic task order, full considered candidate set, unassigned-task blocker metadata, truncation flags, and degraded-signal flags so evidence can choose replay-safe certificate/fallback behavior.

## Validation run

- command: `rg -n "tie-break contract from `scoring.md`|expected_p95_latency_ms when that term is modeled|scheduler task order" internal/scheduler internal/evidence`
- result: pass
- command: `git diff -- internal/scheduler/decision-flow.md internal/scheduler/scoring.md`
- result: pass

## Risks / follow-ups

- open issues:
  - Throughput model for `Assignment.assigned_qps` is specified as policy-hash keyed, but the concrete policy schema/location is out-of-band and must be defined/implemented by the state/policy module.
  - Carbon/power model defaults are deterministic but likely too naive; refine under `policy_hash` without changing replay contract.
- commander decision needed:
  - Confirm whether week-1 should *always* include all enumerated candidates in the bundle (recommended for replay) vs top-K; evidence docs currently assume candidates are required for replay.
