#!/usr/bin/env python3

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class ValidationIssue:
    code: str
    message: str
    related_ids: list[str]


def _collect_declared_ids(items: Any, kind: str, id_field: str) -> tuple[set[str], list[ValidationIssue]]:
    issues: list[ValidationIssue] = []
    ids: set[str] = set()

    if items is None:
        return ids, issues
    if not isinstance(items, list):
        return ids, [ValidationIssue(code=f"snapshot.{kind}_not_array", message=f"{kind} must be an array", related_ids=[])]

    for idx, item in enumerate(items):
        if not isinstance(item, dict):
            issues.append(
                ValidationIssue(code=f"{kind[:-1]}.not_object", message=f"{kind}[{idx}] must be an object", related_ids=[f"{kind}[{idx}]"])
            )
            continue

        item_id = item.get(id_field)
        if not item_id:
            issues.append(
                ValidationIssue(code=f"{kind[:-1]}.missing_id", message=f"{kind}[{idx}] missing {id_field}", related_ids=[])
            )
            continue

        ids.add(str(item_id))

    return ids, issues


def validate_snapshot_payload(snapshot_payload: Any) -> list[ValidationIssue]:
    issues: list[ValidationIssue] = []

    if not isinstance(snapshot_payload, dict):
        return [ValidationIssue(code="snapshot.not_object", message="snapshot payload must be a JSON object", related_ids=[])]

    snapshot_id = snapshot_payload.get("snapshotId")
    if not snapshot_id:
        issues.append(ValidationIssue(code="snapshot.missing_snapshot_id", message="missing snapshotId", related_ids=[]))

    cluster_ids, cluster_issues = _collect_declared_ids(snapshot_payload.get("clusters"), "clusters", "clusterId")
    issues.extend(cluster_issues)

    fault_domain_ids, fault_domain_issues = _collect_declared_ids(
        snapshot_payload.get("faultDomains"),
        "faultDomains",
        "faultDomainId",
    )
    issues.extend(fault_domain_issues)

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
        fault_domain_id = node.get("faultDomain")
        if not node_id:
            issues.append(ValidationIssue(code="node.missing_node_id", message=f"node[{idx}] missing nodeId", related_ids=[]))
        else:
            node_ids.add(str(node_id))

        if not cluster_id:
            issues.append(
                ValidationIssue(code="node.missing_cluster_id", message=f"node[{idx}] missing clusterId", related_ids=[str(node_id or "")])
            )
        elif cluster_ids and str(cluster_id) not in cluster_ids:
            issues.append(
                ValidationIssue(
                    code="node.unknown_cluster_id",
                    message=f"unknown clusterId: {cluster_id}",
                    related_ids=[str(cluster_id)],
                )
            )

        if fault_domain_id and fault_domain_ids and str(fault_domain_id) not in fault_domain_ids:
            issues.append(
                ValidationIssue(
                    code="node.unknown_fault_domain",
                    message=f"unknown faultDomain: {fault_domain_id}",
                    related_ids=[str(fault_domain_id)],
                )
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
