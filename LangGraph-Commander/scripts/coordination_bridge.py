from __future__ import annotations

import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
APP_DIR = ROOT / "local_tools" / "commander-tui"
RUNTIME_DIR = ROOT / "LangGraph-Commander" / "runtime"

sys.path.insert(0, str(APP_DIR))

from coordination_runtime import build_runtime  # type: ignore


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: python LangGraph-Commander/scripts/coordination_bridge.py <intake|approve|review|report> [worker|all]", file=sys.stderr)
        return 2

    runtime = build_runtime(ROOT, RUNTIME_DIR)
    command = argv[0].strip().lower()

    if command == "intake":
        print(runtime.intake(), flush=True)
        return 0
    if command == "approve":
        print(runtime.approve(), flush=True)
        return 0
    if command == "review":
        target = argv[1].strip() if len(argv) > 1 else "all"
        print(runtime.review(target), flush=True)
        return 0
    if command == "report":
        print(runtime.report(), flush=True)
        return 0

    print(f"unknown coordination command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
