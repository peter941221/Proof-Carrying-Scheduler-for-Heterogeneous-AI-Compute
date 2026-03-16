# State fixtures

Fixtures in this directory are intentionally small, deterministic examples for snapshot normalization, fail-fast validation, and hash verification.

## File conventions

- `*.snapshot.json`: valid snapshot payloads aligned to protobuf JSON names from `api/proto/pcs/v1/scheduler.proto`
- `*.snapshot.invalid.json`: intentionally invalid payloads that must fail assembly before hashing or scheduling

## Fixture list

- `mixed_cpu_gpu.v1.snapshot.json`
  - valid mixed CPU-only and GPU nodes in one cluster
  - includes one topology edge so `networkEdges[]` is inside the hash boundary
  - keeps node ordering explicit for a small baseline fixture
- `topology_multi_zone.v1.snapshot.json`
  - valid multi-region, multi-zone topology example
  - includes multiple edges to exercise deterministic edge ordering by `srcId` then `dstId`
  - includes `ResourceVector.ext` map content to exercise canonical object-key sorting
- `unknown_node_ref.v1.snapshot.invalid.json`
  - invalid fixture where `networkEdges[].dstId` references an unknown node
  - defines the required fail-fast behavior before hashing and before scheduling

## What verification checks

`internal/state/tools/verify_fixtures.py` checks that:

- valid fixtures pass structural/reference validation
- valid fixtures already use deterministic node ordering
- valid fixtures already use deterministic edge ordering
- valid fixtures contain the expected `snapshotHash`
- computed hashes match canonical JSON rules with `snapshotHash` stripped
- invalid fixtures fail validation

## Verify locally

From the repo root:

```powershell
python internal/state/tools/verify_fixtures.py
```

To compute a snapshot hash directly:

```powershell
python internal/state/tools/canonical_hash.py --mode snapshot --in internal/state/fixtures/mixed_cpu_gpu.v1.snapshot.json
```
