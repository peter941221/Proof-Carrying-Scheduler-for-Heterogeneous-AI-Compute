#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from typing import Any, Iterable


def _strip_keys(value: Any, keys_to_strip: set[str]) -> Any:
    if isinstance(value, dict):
        return {k: _strip_keys(v, keys_to_strip) for k, v in value.items() if k not in keys_to_strip}
    if isinstance(value, list):
        return [_strip_keys(v, keys_to_strip) for v in value]
    return value


def _canonicalize(value: Any) -> Any:
    if isinstance(value, dict):
        canonical_items: list[tuple[str, Any]] = []
        for key, child in value.items():
            if child is None:
                continue
            canonical_items.append((key, _canonicalize(child)))
        canonical_items.sort(key=lambda kv: kv[0])
        return {k: v for k, v in canonical_items}
    if isinstance(value, list):
        return [_canonicalize(v) for v in value]
    return value


def canonical_json_bytes(value: Any) -> bytes:
    canonical = _canonicalize(value)
    text = json.dumps(
        canonical,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=False,
    )
    return text.encode("utf-8")


def sha256_prefixed(data: bytes) -> str:
    digest = hashlib.sha256(data).hexdigest()
    return f"sha256:{digest}"


def _load_json(path: str) -> Any:
    if path == "-":
        return json.load(sys.stdin)
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def _write_text(path: str | None, text: str) -> None:
    if path is None or path == "-":
        sys.stdout.write(text)
        if not text.endswith("\n"):
            sys.stdout.write("\n")
        return
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
        if not text.endswith("\n"):
            f.write("\n")


def _parse_strip_keys(keys: Iterable[str]) -> set[str]:
    result: set[str] = set()
    for key in keys:
        if not key:
            continue
        result.add(key)
    return result


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Compute canonical JSON and sha256:<hex> hashes for PCS snapshot/bundle payloads.",
    )
    parser.add_argument(
        "--in",
        dest="input_path",
        default="-",
        help="Input JSON file path, or '-' for stdin.",
    )
    parser.add_argument(
        "--mode",
        choices=["raw", "snapshot", "bundle"],
        default="raw",
        help="Hashing policy preset. 'snapshot' strips snapshotHash; 'bundle' strips bundleHash and signature.",
    )
    parser.add_argument(
        "--strip-key",
        action="append",
        default=[],
        help="Additional JSON object keys to strip everywhere before canonicalization (repeatable).",
    )
    parser.add_argument(
        "--print-canonical",
        action="store_true",
        help="Print canonical JSON to stdout (instead of just the hash).",
    )
    parser.add_argument(
        "--out-canonical",
        default=None,
        help="Write canonical JSON to this path (or '-' for stdout).",
    )
    args = parser.parse_args(argv)

    payload = _load_json(args.input_path)

    keys_to_strip = _parse_strip_keys(args.strip_key)
    if args.mode == "snapshot":
        keys_to_strip |= {"snapshotHash"}
    elif args.mode == "bundle":
        keys_to_strip |= {"bundleHash", "signature"}

    stripped = _strip_keys(payload, keys_to_strip) if keys_to_strip else payload
    canonical_bytes = canonical_json_bytes(stripped)
    canonical_text = canonical_bytes.decode("utf-8")

    if args.out_canonical is not None:
        _write_text(args.out_canonical, canonical_text)

    if args.print_canonical:
        _write_text("-", canonical_text)
        return 0

    _write_text("-", sha256_prefixed(canonical_bytes))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

