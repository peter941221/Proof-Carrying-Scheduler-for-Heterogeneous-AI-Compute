# Verifier Claim Matrix

This matrix gives a deterministic verifier-side mapping from claim IDs to required stages, downgrade rules, and stable artifact references.

It is the compact implementation companion to `verification-stages.md` and `claim-reporting.md`.

## Matrix

```text
Claim ID                      Required stages   CHECKED when...                          PLANNED when...                               artifact_refs
VERIFY.BUNDLE_SOUNDNESS       S0-S2             S0-S2 all passed                         any of S0-S2 failed or later blocked output   spec/decision-bundle.md, spec/canonical-json.md, verifier/verification-stages.md
SAFETY.SNAPSHOT_CONSISTENCY   S4                S4 passed after S0-S2 passed            S4 failed, skipped, or blocked                spec/snapshot-contract.md, verifier/verification-stages.md
EVIDENCE.APPEND_ONLY_CHAIN    none (runtime)    never in v0 runtime verifier             chain semantics not implemented in verifier   proofs/tla+/EvidenceChain.tla, proofs/tla+/ASSUMPTIONS.md
VERIFY.CONFLICT_SOUNDNESS     S5                S5 passed for a conflict-check request   S5 failed, skipped, or blocked                spec/decision-bundle.md, verifier/claim-reporting.md
BOUND.RELATIVE_GAP_REPORTING  S6                S6 passed for a bound-check request      S6 failed, skipped, or blocked                spec/decision-bundle.md, verifier/claim-reporting.md
```

## Downgrade rules

- If any prerequisite stage is `failed`, the claim must not exceed `CLAIM_STATUS_PLANNED`.
- If any prerequisite stage is `blocked`, the claim must not exceed `CLAIM_STATUS_PLANNED` and the summary should point to the earlier hard-stop stage.
- If a stage is disabled by request, the claim may be omitted or emitted as `CLAIM_STATUS_PLANNED`, but it must not appear as `CHECKED`.
- Runtime verification must not emit `IMPLEMENTED` or `VERIFIED`; those stronger statuses belong to durable artifacts in `proofs/`.

## Notes

- `EVIDENCE.APPEND_ONLY_CHAIN` remains proof-backed rather than runtime-backed in v0 because current verifier stages do not define explicit chain validation semantics.
- `SAFETY.SNAPSHOT_CONSISTENCY` should be emitted only when S4 passed; if S4 is skipped because no enabled stage requires snapshot material, the claim should be omitted or downgraded to `CLAIM_STATUS_PLANNED`, never `CHECKED`.
- Future signature-related claim IDs should be added here only after the shared claim taxonomy names them.
