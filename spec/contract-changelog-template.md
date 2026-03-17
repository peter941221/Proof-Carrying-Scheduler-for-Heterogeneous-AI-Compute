# Contract Changelog Template

Use this template for future commander-reviewed changes to the frozen contract packet.

## Entry

- date:
- change_id:
- commander_decision:
- status: proposed | approved | rejected | superseded
- surfaces:
  - `api/proto/pcs/v1/scheduler.proto`
- summary:
- reason:
- wire impact: none | additive | breaking
- downstream modules affected:
  - state
  - scheduler-evidence
  - verifier-proofs
- required migrations:
- validation run:
- follow-ups:

## Notes

- Record one entry per contract decision.
- List every added, renamed, or behaviorally reinterpreted field, enum, or spec rule.
- If wire impact is `breaking`, include the rollout and compatibility plan.
