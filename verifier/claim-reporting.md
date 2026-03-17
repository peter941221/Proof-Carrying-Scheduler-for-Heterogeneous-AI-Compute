# Claim Reporting (Verifier Policy)

This document defines how the verifier populates `VerifyResponse.checked_claims` (`ClaimCheck` entries) so that claim language stays aligned with actual exercised checks.

## Goals

- make omissions explicit (disabled stages must not read as success)
- keep claim vocabulary consistent with `spec/claim-taxonomy.md`
- support claim lifecycle (`PLANNED` -> `VERIFIED`) without overstating
- make runtime output deterministic from the final stage outcome map alone

## ClaimCheck population rules

### 1) Only report exercised claims as `CHECKED`

A claim may be marked `CLAIM_STATUS_CHECKED` only if the corresponding stage ran and succeeded under the request policy.

Examples:

- if S2 succeeds, the verifier can mark `VERIFY.BUNDLE_SOUNDNESS` as `CHECKED` for hash/structure soundness
- if S3 is disabled, do not mark signature-related claims as checked
- if S5 is disabled, do not mark constraint soundness claims as checked

### 2) Use `PLANNED` for known-but-not-checked claims (optional)

If the verifier includes claims that were not exercised, they must use `CLAIM_STATUS_PLANNED` and include a `summary` stating why the claim was not checked (e.g., stage disabled, missing snapshot payload).

### 3) Never emit `IMPLEMENTED` or `VERIFIED` from runtime verification

Runtime verification can justify `CHECKED`. Higher statuses must come from durable artifacts (model-check results, theorem assets, etc.) and belong in the proofs module.

### 4) Claim IDs must match the taxonomy

If the verifier encounters a claim ID that is not in `spec/claim-taxonomy.md`:

- include it only with `CLAIM_STATUS_PLANNED`
- emit `CLAIM.UNKNOWN_CLAIM_ID` as a warning

### 5) Synthesize claims after stage outcomes are final

`checked_claims` should be derived only in S8, after all enabled stages either succeed, fail, or are skipped. This keeps claim statuses deterministic and prevents later hard-stops from leaving stale optimistic claim entries in the response.

Recommended synthesis order:

1. collect the final stage outcome map (`passed`, `failed`, `skipped`, `blocked`)
2. map outcomes to claim IDs using `claim-matrix.md`
3. emit `CHECKED` only for claims whose required stages all `passed`
4. emit optional `PLANNED` entries for `failed`, `skipped`, or `blocked` claims with a short reason summary
5. for `skipped` claims, pair the summary with `CLAIM.SKIPPED_STAGE` or the stage-specific `*.SKIPPED` code
6. for `blocked` claims, point the summary at the earlier hard-stop stage and optionally emit `CLAIM.BLOCKED_STAGE`
7. if any produced entry would imply stronger evidence than the stage outcome supports, emit `CLAIM.OVERSTATED_STATUS` and downgrade the claim status

`blocked` means the stage did not run because an earlier hard-stop removed a prerequisite. `blocked` is weaker than `skipped`; both must avoid implying success.

## Recommended initial mapping (v0)

This worktree uses the initial claim IDs from `spec/claim-taxonomy.md` and maps them to verifier stages as follows:

```text
VERIFY.BUNDLE_SOUNDNESS
  - exercised by: S0-S2
  - status: CHECKED on success, else PLANNED with failure or blocked summary

SAFETY.SNAPSHOT_CONSISTENCY
  - exercised by: S4
  - prerequisite: S0-S2 passed
  - status: CHECKED only when snapshot binding is verified

EVIDENCE.APPEND_ONLY_CHAIN
  - exercised by: S2 hash checks plus explicit chain checks when implemented
  - status: PLANNED for now unless chain semantics are implemented

VERIFY.CONFLICT_SOUNDNESS
  - exercised by: S5 on CONFLICT-level bundles with failing constraint evals
  - prerequisite: S4 passed when snapshot-backed replay is required
  - status: CHECKED only when S5 ran and replay confirms the conflict semantics

BOUND.RELATIVE_GAP_REPORTING
  - exercised by: S6 when bound certificate is present and verified
  - prerequisite: S5 passed if the bound depends on replayed semantics
  - status: CHECKED only when S6 ran
```

For compact implementation guidance, keep `claim-matrix.md` and this section aligned.

## `artifact_refs` guidance

`artifact_refs` in verifier output should reference stable artifacts or evidence items, for example:

- `spec/decision-bundle.md`
- `spec/canonical-json.md`
- `spec/verification-report.md`
- a signer key identifier or trust policy reference (when defined)

Keep `artifact_refs` short and durable. Prefer repo-relative paths when possible.
