from __future__ import annotations

import json
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
APP_DIR = ROOT / "local_tools" / "commander-tui"
RUNTIME_DIR = ROOT / "LangGraph-Commander" / "runtime"

sys.path.insert(0, str(APP_DIR))

from langgraph_runtime import LangGraphWorkerRunner, WorkerConfig  # type: ignore


def load_worker(name: str) -> WorkerConfig:
    payload = json.loads((APP_DIR / "workers.json").read_text(encoding="utf-8"))
    for item in payload["workers"]:
        if item["name"] == name:
            return WorkerConfig.from_dict(item)
    raise SystemExit(f"unknown worker: {name}")


def main(argv: list[str]) -> int:
    if len(argv) != 1:
        print("usage: python LangGraph-Commander/scripts/worker_bridge.py <worker-name>", file=sys.stderr)
        return 2

    worker_name = argv[0]
    config = load_worker(worker_name)

    def log(message: str) -> None:
        stamp = time.strftime("%Y-%m-%d %H:%M:%S")
        print(f"{stamp} [{worker_name}] {message}", flush=True)

    runner = LangGraphWorkerRunner(
        repo_root=ROOT,
        runtime_dir=RUNTIME_DIR,
        config=config,
        log_callback=log,
    )
    runner.start()

    while runner.is_running():
        time.sleep(1.0)

    state = runner.load_state()
    status = str(state.get("status") or "")
    if status != "finished":
        print(json.dumps(state, ensure_ascii=False, indent=2), file=sys.stderr)
        return 1

    summary = str(state.get("last_summary") or "").strip()
    if summary:
        print(summary, flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
