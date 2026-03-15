# Spec Module

This directory holds the human-readable contract layer behind the wire format.

## Owns

- bundle semantics
- canonical JSON and hash rules
- snapshot boundaries
- verification report semantics
- claim taxonomy and lifecycle

## Source of truth

- `contract-packet.md`
- `decision-bundle.md`
- `canonical-json.md`
- `snapshot-contract.md`
- `verification-report.md`
- `claim-taxonomy.md`
- `claim-lifecycle.md`

## For module agents

If you open the `module/api-spec` worktree, use this directory to answer:

- what each shared object means
- which fields are hashed or signed
- which guarantees can be claimed publicly

## Must preserve

- one authoritative definition per shared concept
- wording that matches actual verifier or proof boundaries
- clear separation between modeled, checked, implemented, and verified claims
