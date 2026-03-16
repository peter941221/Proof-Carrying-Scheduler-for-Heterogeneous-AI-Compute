#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import dataclass
from typing import Any

from canonical_hash import canonical_json_bytes, sha256_prefixed
from validate_snapshot import validate_snapshot_payload


@dataclass(frozen=True)
class CheckResult:
    path: str
    ok: bool
    message: str


def _load_json(path: str) -> Any:
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _compute_snapshot_hash(snapshot_payload: Any) -> str:
    if not isinstance(snapshot_payload, dict):
        raise ValueError("snapshot payload must be a JSON object")

    without_hash = {k: v for k, v in snapshot_payload.items() if k != "snapshotHash"}
    return sha256_prefixed(canonical_json_bytes(without_hash))


def _node_sort_key(node: dict[str, Any]) -> tuple[str, str, str, str]:
    return (
        str(node.get("clusterId") or ""),
        str(node.get("region") or ""),
        str(node.get("zone") or ""),
        str(node.get("nodeId") or ""),
    )


def _edge_sort_key(edge: dict[str, Any]) -> tuple[str, str]:
    return (str(edge.get("srcId") or ""), str(edge.get("dstId") or ""))


def _check_normalized_order(snapshot_payload: dict[str, Any]) -> str | None:
    nodes = snapshot_payload.get("nodes") or []
    if isinstance(nodes, list):
        sorted_nodes = sorted(nodes, key=_node_sort_key)
        if nodes != sorted_nodes:
            return "nodes not sorted by clusterId, region, zone, nodeId"

    edges = snapshot_payload.get("networkEdges") or []
    if isinstance(edges, list):
        sorted_edges = sorted(edges, key=_edge_sort_key)
        if edges != sorted_edges:
            return "networkEdges not sorted by srcId, dstId"

    return None


def _check_snapshot_fixture(path: str) -> CheckResult:
    try:
        payload = _load_json(path)
        issues = validate_snapshot_payload(payload)
        if issues:
            return CheckResult(path=path, ok=False, message=f"validation failed: {issues[0].code} ({issues[0].message})")

        ordering_issue = _check_normalized_order(payload)
        if ordering_issue is not None:
            return CheckResult(path=path, ok=False, message=ordering_issue)

        expected = payload.get("snapshotHash")
        if not expected:
            return CheckResult(path=path, ok=False, message="missing snapshotHash")

        actual = _compute_snapshot_hash(payload)
        if actual != expected:
            return CheckResult(path=path, ok=False, message=f"hash mismatch: expected={expected} actual={actual}")
        return CheckResult(path=path, ok=True, message="ok")
    except Exception as e:  # noqa: BLE001
        return CheckResult(path=path, ok=False, message=str(e))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Verify internal/state snapshot fixtures.")
    parser.add_argument(
        "--fixtures-dir",
        default=os.path.join(os.path.dirname(__file__), "..", "fixtures"),
        help="Fixtures directory to scan.",
    )
    args = parser.parse_args(argv)

    fixtures_dir = os.path.abspath(args.fixtures_dir)
    if not os.path.isdir(fixtures_dir):
        print(f"fixtures dir not found: {fixtures_dir}", file=sys.stderr)
        return 2

    failures: list[CheckResult] = []
    checked = 0
    invalid_checked = 0
    invalid_failures: list[CheckResult] = []

    for name in sorted(os.listdir(fixtures_dir)):
        if not name.endswith(".snapshot.json"):
            if name.endswith(".snapshot.invalid.json"):
                invalid_checked += 1
                path = os.path.join(fixtures_dir, name)
                try:
                    payload = _load_json(path)
                    issues = validate_snapshot_payload(payload)
                    if not issues:
                        invalid_failures.append(CheckResult(path=path, ok=False, message="expected validation failure, got none"))
                except Exception as e:  # noqa: BLE001
                    invalid_failures.append(CheckResult(path=path, ok=False, message=str(e)))
            continue

        checked += 1
        path = os.path.join(fixtures_dir, name)
        result = _check_snapshot_fixture(path)
        if not result.ok:
            failures.append(result)

    if checked == 0:
        print("no *.snapshot.json fixtures found", file=sys.stderr)
        return 2

    if failures:
        for f in failures:
            print(f"FAIL {os.path.basename(f.path)}: {f.message}", file=sys.stderr)
        return 1

    if invalid_failures:
        for f in invalid_failures:
            print(f"FAIL {os.path.basename(f.path)}: {f.message}", file=sys.stderr)
        return 1

    print(f"PASS ({checked} valid fixtures, {invalid_checked} invalid fixtures)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
