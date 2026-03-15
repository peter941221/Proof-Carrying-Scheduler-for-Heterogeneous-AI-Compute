# Claim Lifecycle

## Status vocabulary

- `CLAIM_STATUS_PLANNED`
- `CLAIM_STATUS_MODELED`
- `CLAIM_STATUS_CHECKED`
- `CLAIM_STATUS_IMPLEMENTED`
- `CLAIM_STATUS_VERIFIED`

## Meanings

### Planned

The claim exists in project scope but has no artifact yet.

### Modeled

The claim is represented in a spec, invariant list, or formal model draft.

### Checked

The claim is exercised by a runtime checker, verifier path, replay, or test harness.

### Implemented

The necessary product behavior exists in code, but formal or independent verification may still be incomplete.

### Verified

A durable artifact exists that justifies the claim at the stated boundary, such as:

- a model-check result
- a theorem artifact
- a proof-oriented checker result
- an independent verifier report tied to the claim

## Reporting rule

Project docs and demos must always speak at the highest status actually achieved, never higher.
