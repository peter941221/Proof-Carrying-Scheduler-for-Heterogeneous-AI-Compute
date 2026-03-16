#!/usr/bin/env python3

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class ValidationIssue:
    code: str
    message: str
    related_ids: list[str]


def validate_snapshot_payload(snapshot_payload: Any) -> list[ValidationIssue]:
    issues: list[ValidationIssue] = []

    if not isinstance(snapshot_payload, dict):
        return [ValidationIssue(code="snapshot.not_object", message="snapshot payload must be a JSON object", related_ids=[])]

    snapshot_id = snapshot_payload.get("snapshotId")
    if not snapshot_id:
        issues.append(ValidationIssue(code="snapshot.missing_snapshot_id", message="missing snapshotId", related_ids=[]))

    nodes = snapshot_payload.get("nodes") or []
    if not isinstance(nodes, list):
        issues.append(ValidationIssue(code="snapshot.nodes_not_array", message="nodes must be an array", related_ids=[]))
        nodes = []

    node_ids: set[str] = set()
    for idx, node in enumerate(nodes):
        if not isinstance(node, dict):
            issues.append(
                ValidationIssue(code="node.not_object", message=f"node[{idx}] must be an object", related_ids=[f"nodes[{idx}]"])
            )
            continue

        node_id = node.get("nodeId")
        cluster_id = node.get("clusterId")
        if not node_id:
            issues.append(ValidationIssue(code="node.missing_node_id", message=f"node[{idx}] missing nodeId", related_ids=[]))
        else:
            node_ids.add(str(node_id))

        if not cluster_id:
            issues.append(
                ValidationIssue(code="node.missing_cluster_id", message=f"node[{idx}] missing clusterId", related_ids=[str(node_id or "")])
            )

    edges = snapshot_payload.get("networkEdges") or []
    if not isinstance(edges, list):
        issues.append(ValidationIssue(code="snapshot.edges_not_array", message="networkEdges must be an array", related_ids=[]))
        edges = []

    for idx, edge in enumerate(edges):
        if not isinstance(edge, dict):
            issues.append(
                ValidationIssue(code="edge.not_object", message=f"networkEdges[{idx}] must be an object", related_ids=[f"networkEdges[{idx}]"])
            )
            continue

        src = edge.get("srcId")
        dst = edge.get("dstId")
        if not src:
            issues.append(ValidationIssue(code="edge.missing_src", message=f"networkEdges[{idx}] missing srcId", related_ids=[]))
        if not dst:
            issues.append(ValidationIssue(code="edge.missing_dst", message=f"networkEdges[{idx}] missing dstId", related_ids=[]))

        if src and str(src) not in node_ids:
            issues.append(
                ValidationIssue(code="edge.unknown_src", message=f"unknown srcId: {src}", related_ids=[str(src)])
            )
        if dst and str(dst) not in node_ids:
            issues.append(
                ValidationIssue(code="edge.unknown_dst", message=f"unknown dstId: {dst}", related_ids=[str(dst)])
            )

    return issues

