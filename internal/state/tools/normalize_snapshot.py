#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import sys
from typing import Any


def _canonicalize_nulls(value: Any) -> Any:
    if isinstance(value, dict):
        result: dict[str, Any] = {}
        for key, child in value.items():
            if child is None:
                continue
            result[key] = _canonicalize_nulls(child)
        return result
    if isinstance(value, list):
        return [_canonicalize_nulls(child) for child in value]
    return value


def _node_sort_key(node: Any) -> tuple[str, str, str, str]:
    if not isinstance(node, dict):
        return ("", "", "", "")
    return (
        str(node.get("clusterId") or ""),
        str(node.get("region") or ""),
        str(node.get("zone") or ""),
        str(node.get("nodeId") or ""),
    )


def _edge_sort_key(edge: Any) -> tuple[str, str]:
    if not isinstance(edge, dict):
        return ("", "")
    return (str(edge.get("srcId") or ""), str(edge.get("dstId") or ""))


def _cluster_sort_key(cluster: Any) -> tuple[str]:
    if not isinstance(cluster, dict):
        return ("",)
    return (str(cluster.get("clusterId") or ""),)


def _fault_domain_sort_key(fault_domain: Any) -> tuple[str]:
    if not isinstance(fault_domain, dict):
        return ("",)
    return (str(fault_domain.get("faultDomainId") or ""),)


def normalize_snapshot_payload(snapshot_payload: Any) -> Any:
    if not isinstance(snapshot_payload, dict):
        raise ValueError("snapshot payload must be a JSON object")

    normalized = _canonicalize_nulls(snapshot_payload)

    nodes = normalized.get("nodes")
    if isinstance(nodes, list):
        normalized["nodes"] = sorted(nodes, key=_node_sort_key)

    edges = normalized.get("networkEdges")
    if isinstance(edges, list):
        normalized["networkEdges"] = sorted(edges, key=_edge_sort_key)

    clusters = normalized.get("clusters")
    if isinstance(clusters, list):
        normalized["clusters"] = sorted(clusters, key=_cluster_sort_key)

    fault_domains = normalized.get("faultDomains")
    if isinstance(fault_domains, list):
        normalized["faultDomains"] = sorted(fault_domains, key=_fault_domain_sort_key)

    return normalized


def _load_json(path: str) -> Any:
    if path == "-":
        return json.load(sys.stdin)
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _write_json(path: str | None, payload: Any) -> None:
    text = json.dumps(payload, ensure_ascii=False, indent=2)
    if path is None or path == "-":
        sys.stdout.write(text)
        if not text.endswith("\n"):
            sys.stdout.write("\n")
        return
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
        if not text.endswith("\n"):
            f.write("\n")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Normalize snapshot payload ordering before hashing.")
    parser.add_argument("--in", dest="input_path", default="-", help="Input JSON file path, or '-' for stdin.")
    parser.add_argument("--out", dest="output_path", default="-", help="Output JSON file path, or '-' for stdout.")
    args = parser.parse_args(argv)

    payload = _load_json(args.input_path)
    normalized = normalize_snapshot_payload(payload)
    _write_json(args.output_path, normalized)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
