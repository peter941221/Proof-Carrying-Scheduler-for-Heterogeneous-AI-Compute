# Evidence Handoff

## Status

- ready

## Summary

- Locked deterministic DecisionBundle assembly rules: repeated-field ordering, structure checks, canonical-json hashing flow, and explicit certificate/fallback/bound emission policy for week-1.
- Expanded ConstraintEval witness requirements for partial/unassigned tasks, fully infeasible batches, and candidate-budget truncation cases.
- Clarified certificate-level defaults for partial and infeasible outputs, plus omission rules for normal-path `fallback` and unused `counterfactuals`/shadow analysis fields.
- Resolved the week-1 FEASIBLE-certificate ambiguity by explicitly allowing `DECISION_STATUS_INFEASIBLE` only when the implementation is not making a stronger `CERTIFICATE_LEVEL_CONFLICT` claim.

## Files changed

- `internal/evidence/evidence-emission.md`
- `internal/evidence/constraint-evals.md`

## Witness coverage

- feasible path: minimum satisfied ConstraintEval set for resource/compat/placement/budget/carbon; candidates required for replay; deterministic ordering enforced for stable hashing.
- partial path: requires at least one failing ConstraintEval per unassigned task (`subject_id = task_id`) plus fallback emission; distinguishes “not fully explored” via `policy.candidate_budget` + `FALLBACK_CODE_SOLVER_BUDGET_EXCEEDED`.
- infeasible path: requires conflict witness ConstraintEval(s) for every failed task, even when no assignments exist; supports “no candidate” by emitting the tightest discovered blocker category.
- optional payload path: `counterfactuals` and shadow-level outputs remain allowed but must be intentionally produced; otherwise evidence omits those fields and certificate levels entirely.

## Validation run

- command: `rg -n "CERTIFICATE_LEVEL_CONFLICT|FALLBACK_CODE_NONE|DECISION_STATUS_INFEASIBLE|counterfactuals field entirely|Fully infeasible batch witness" internal/evidence internal/scheduler`
- result: pass
- command: `git diff -- internal/evidence/evidence-emission.md internal/evidence/constraint-evals.md`
- result: pass

## Risks / follow-ups

- open issues:
  - Evidence builder must treat reserved `subject_id = system` as valid even without a concrete bundle object; verifier must mirror this convention.
  - Candidate-set size may be large; if top-K is introduced later it must be explicit and replay-safe.
- commander decision needed:
  - Confirm week-1 stance on including `bound` only when claiming `CERTIFICATE_LEVEL_BOUNDED` (current doc: yes).
