# Canonical JSON and Hashing

## Canonicalization rules

- field names use the protobuf JSON name
- object keys are sorted lexicographically
- maps are emitted with sorted keys
- arrays preserve semantic order from the bundle payload
- numbers are emitted in a single deterministic form (recommendation: RFC 8785 / JCS number formatting)
- bytes fields are base64 encoded before hashing
- timestamps use the protobuf JSON mapping in UTC with a `Z` suffix (producers must not emit equivalent-but-different encodings)
- absent optional fields are omitted, not serialized as `null`

## Hash scope

`bundle_hash` is computed over the canonical JSON form of `DecisionBundle` with:

- `bundle_hash` omitted
- `signature` omitted

## Chain scope

`prev_bundle_hash` is part of the hash input when present, so chain linkage is covered by the next bundle hash.

## Snapshot scope

The snapshot itself is not embedded in the bundle hash; the bundle binds to it through `snapshot_ref.snapshot_hash`.

## Sign-then-store rule

1. build bundle payload
2. canonicalize JSON
3. compute `bundle_hash`
4. sign the hash or canonical bytes under the configured signer policy
5. persist bundle and append to the evidence log
