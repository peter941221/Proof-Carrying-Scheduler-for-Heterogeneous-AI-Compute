# Evidence Handoff

## Status

- ready

## Summary

- Locked deterministic DecisionBundle assembly rules: repeated-field ordering, structure checks, chain-link placement, canonical-json hashing flow, and explicit certificate/fallback/bound emission policy for week-1.
- Expanded ConstraintEval witness requirements for partial/unassigned tasks, fully infeasible batches, and candidate-budget truncation cases.
- Clarified certificate-level defaults for partial and infeasible outputs, plus omission rules for normal-path `fallback` and unused `counterfactuals`/shadow analysis fields.
- Tightened referential and ordering rules by pinning `system` as the reserved global subject ID, requiring candidate IDs in closest-to-feasible witnesses when relevant, and aligning candidate/assignment sorting with scheduler task order.

## Files changed

- `internal/evidence/evidence-emission.md`
- `internal/evidence/constraint-evals.md`

## Witness coverage

- feasible path: minimum satisfied ConstraintEval set for resource/compat/placement/budget/carbon; candidates required for replay; deterministic ordering enforced for stable hashing.
- partial path: requires at least one failing ConstraintEval per unassigned task (`subject_id = task_id`) plus fallback emission; distinguishes “not fully explored” via `policy.candidate_budget` + `FALLBACK_CODE_SOLVER_BUDGET_EXCEEDED`.
- infeasible path: requires conflict witness ConstraintEval(s) for every failed task, even when no assignments exist; supports “no candidate” by emitting the tightest discovered blocker category.
- bundle-finalization path: `prev_bundle_hash` must be set before hashing when chaining is enabled, `created_at` is finalized last, and `counterfactuals` / shadow-level outputs remain omitted unless intentionally produced.

## Validation run

- command: `rg -n "Set chain linkage|scheduler task order first|candidate_id when the witness|subject ID `system`" internal/evidence internal/scheduler`
- result: pass
- command: `git diff -- internal/evidence/evidence-emission.md internal/evidence/constraint-evals.md`
- result: pass

## Risks / follow-ups

- open issues:
  - Evidence builder must treat reserved `subject_id = system` as valid even without a concrete bundle object; verifier must mirror this convention.
  - Candidate-set size may be large; if top-K is introduced later it must be explicit and replay-safe.
- commander decision needed:
  - Confirm week-1 stance on including `bound` only when claiming `CERTIFICATE_LEVEL_BOUNDED` (current doc: yes).
