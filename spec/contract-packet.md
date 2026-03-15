# Contract Packet

## Frozen surfaces

- `api/proto/pcs/v1/scheduler.proto`
- `spec/decision-bundle.md`
- `spec/snapshot-contract.md`
- `spec/canonical-json.md`
- `spec/verification-report.md`
- `spec/claim-taxonomy.md`
- `spec/claim-lifecycle.md`

## Commander gate

Any change to these surfaces requires commander review before module implementation continues:

- protobuf fields, enum values, RPC names, or package paths
- canonical serialization order and hash inputs
- signature envelope fields
- certificate level semantics
- verification issue codes and trust-boundary wording

## Initial freeze goal

The first freeze establishes:

- `DecisionBundle` as the evidence source of truth
- deterministic snapshot metadata and hash boundaries
- snapshot, model, policy, and solver config hash references
- deterministic score and constraint witness payloads
- verifier input/output contracts and claim status vocabulary that remain stable while modules iterate
