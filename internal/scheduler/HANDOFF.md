# Scheduler Handoff

## Status

- ready

## Summary

- Defined deterministic end-to-end scheduler flow from request intake through evidence handoff, including replay-safe IDs, stable ordering, and explicit fallback triggers.
- Locked scoring semantics for week-1 with ordered objective terms, normalization rules, and deterministic tie-break behavior.
- Tightened scheduler-to-evidence boundaries so the evidence builder can justify emitted fields without hidden scheduler-only state.

## Files changed

- `internal/scheduler/decision-flow.md`
- `internal/scheduler/scoring.md`

## Decision-path coverage

- training path:
  - resource-fit candidate generation, ordered score contributions, deterministic assignment ordering, and fallback triggers for exhausted search or solver budget.
- inference path:
  - latency-sensitive candidate pruning, stable tie-breaks, fallback emission when the scheduler exits early, and evidence-facing decision context requirements.
- deterministic replay:
  - candidate IDs, assignment ordering, omitted-term handling, and final decision status transitions are explicit.

## Validation run

- command: `rg -n "Evidence-facing decision context|FALLBACK_CODE_NONE|deterministic tie-break|DECISION_STATUS_(FEASIBLE|PARTIAL|INFEASIBLE)" internal/scheduler internal/evidence`
- result: pass
- command: `git diff -- internal/scheduler/decision-flow.md internal/scheduler/scoring.md`
- result: pass

## Risks / follow-ups

- open issues:
  - If a future optimizer introduces stochastic exploration or top-K truncation, the truncation and randomness controls must be surfaced explicitly in replayable policy inputs.
  - Scheduler docs currently assume snapshot normalization is already complete; implementation must fail before scheduling on invalid snapshot references.
- commander decision needed:
  - Confirm whether week-1 should keep tie-break semantics entirely lexical / stable, or reserve an explicit deterministic “priority seed” field for later milestones.
