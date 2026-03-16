# State fixtures

Fixtures in this directory are intended to be small, deterministic examples that exercise snapshot normalization and hashing rules.

## Files

- `*.snapshot.json` are snapshot-shaped payloads aligned with the protobuf JSON names in `api/proto/pcs/v1/scheduler.proto` (`SnapshotMetadata`).
- `*.snapshot.invalid.json` are intentionally invalid inputs used to define fail-fast behavior (they are not included in the hash verification pass).

### Fixture list (what each tests)

- `mixed_cpu_gpu.v1.snapshot.json`
  - mixed CPU-only and GPU nodes in one cluster
  - includes a simple edge to ensure `networkEdges[]` participates in the hash boundary
- `topology_multi_zone.v1.snapshot.json`
  - multiple zones and regions
  - multiple edges (tests deterministic edge ordering by `srcId` then `dstId`)
  - includes a `ResourceVector.ext` map to exercise canonical map key sorting
- `unknown_node_ref.v1.snapshot.invalid.json`
  - invalid: a `networkEdges[].dstId` references an unknown node ID
  - should fail snapshot assembly before hashing/scheduling (fail-fast behavior)

## Verify snapshot hashes

From the repo root:

```powershell
python internal/state/tools/verify_fixtures.py
```

This checks:

- valid fixtures:
  - pass fail-fast validation (unknown references, missing IDs)
  - have correct `snapshotHash` matching canonical JSON hashing rules
- invalid fixtures:
  - must fail fail-fast validation (at least one validation issue)

To compute a hash for a snapshot payload (stripping `snapshotHash` before hashing):

```powershell
python internal/state/tools/canonical_hash.py --mode snapshot --in internal/state/fixtures/mixed_cpu_gpu.v1.snapshot.json
```
