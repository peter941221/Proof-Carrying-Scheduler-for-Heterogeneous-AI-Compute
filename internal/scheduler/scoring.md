# Scheduler Scoring (deterministic) (v0)

This document defines how the scheduler computes and orders objective terms so the evidence plane can
emit replayable `ObjectiveTerm` payloads.

It must remain consistent with:

- `api/proto/pcs/v1/scheduler.proto` (`ObjectiveTerm`, `Candidate`, `Assignment`)
- `spec/decision-bundle.md` (replay expectations)

## Principle

- Every score component that influences selection must appear in `objective_terms`.
- `objective_terms` order is **semantic** and must be deterministic.
- The verifier should be able to recompute contributions from bundle + snapshot inputs; avoid hidden heuristics.

## Term ordering (week-1)

Emit terms in this exact order when applicable:

1. `urgency`
2. `cost_usd_per_hour`
3. `carbon_gco2_per_hour`
4. `latency_p95_ms`
5. `interruption_risk`
6. `topology_score`

If a term is not modeled for a decision, omit it (do not emit placeholder zeros).

## Normalization rules

For each term:

- `raw_value`: the unnormalized value in natural units
- `normalized_value`: mapped into `[0, 1]` where larger means “better”
- `weight`: configured scalar weight
- `contribution`: `normalized_value * weight`

### Policy-defined ranges (required for replay)

Normalization may **not** depend on “the set of candidates considered” unless the bounds are also
committed to the bundle. Because `ObjectiveTerm` does not carry bounds, week-1 normalization ranges
must be derived from the policy configuration identified by `SnapshotRef.policy_hash` (and/or the
solver config identified by `SnapshotRef.solver_config_hash`).

The verifier is expected to load the same policy config by hash and recompute `normalized_value`
from `raw_value` deterministically.

### Week-1 default normalization functions

Let `clamp01(x) = min(1, max(0, x))`.

For terms where “lower is better”, the normalization uses an affine map from a policy range
`[min, max]`:

- if `max <= min`, treat the term as **not modelable** for this decision and require degraded
  fallback (`FALLBACK_CODE_DEGRADED_SIGNAL`) rather than emitting a misleading normalized value.
- otherwise: `normalized = 1 - clamp01((raw - min) / (max - min))`

For terms where “higher is better”, use:

- `normalized = clamp01((raw - min) / (max - min))`

If a term is enabled by policy but `raw_value` cannot be computed from snapshot + task + candidate
fields, the scheduler must emit degraded fallback (do not silently omit the term).

Default week-1 term directions and suggested policy ranges:

- `urgency` (higher is better): range `[0, 1]` (treat `Task.urgency` as already normalized).
- `cost_usd_per_hour` (lower is better): range from policy (e.g., `[0, 1000]`).
- `carbon_gco2_per_hour` (lower is better): range from policy.
- `latency_p95_ms` (lower is better): range from policy.
- `interruption_risk` (lower is better): range from policy.
- `topology_score` (higher is better): range `[0, 1]`.

## Total objective (week-1)

Total objective contribution is the sum of `ObjectiveTerm.contribution` across the emitted terms
(terms omitted because they are disabled by policy do not contribute).

## Tie-breaking contract

When two candidates or assignments have equal total contribution within a configured epsilon:

1. prefer the one with lower `expected_cost_usd_per_hour`
2. then prefer lower `expected_carbon_gco2_per_hour`
3. then prefer lower `expected_p95_latency_ms` when that term is modeled for both options
4. then prefer lexicographically smaller `candidate_id` / `assignment_id`

The epsilon itself must be policy-defined and replayable via `SnapshotRef.policy_hash`; if no epsilon
is configured, use exact floating-point comparison of the emitted `contribution` values.

If tie-breaking changes, treat it as contract-impacting because it changes replay determinism.
