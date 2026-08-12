#!/usr/bin/env python3
"""Convert an operator-owned BIOS/boot log into generic ``keygen.story.v1``.

This is intentionally a metadata bridge, not a content extractor.  Input files
must be supplied explicitly and output is refused inside the repository.  The
result may contain recovered text, so keep it in a private temporary directory
and never commit it.
"""
from __future__ import annotations

import argparse
import json
import re
import tempfile
from pathlib import Path

TIMESTAMP = re.compile(r"^\s*(?P<t>(?:\d+(?:\.\d+)?))\s*(?:s|sec|seconds)?\s*[:|,-]\s*(?P<text>.+)$", re.I)
BRACKET = re.compile(r"^\s*\[(?P<t>\d+(?:\.\d+)?)\]\s*(?P<text>.+)$")
TIMING_ONLY = re.compile(r"^\s*\[(?P<t>\d+(?:\.\d+)?)\]\s*$")


def parse(path: Path, limit: int) -> list[tuple[float, str]]:
    rows: list[tuple[float, str]] = []
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if len(rows) >= limit:
            break
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        timing_only = TIMING_ONLY.match(line)
        if timing_only:
            rows.append((float(timing_only.group("t")), ""))
            continue
        match = TIMESTAMP.match(line) or BRACKET.match(line)
        if match:
            rows.append((float(match.group("t")), match.group("text").strip()))
        else:
            # Untimed log lines remain useful metadata and execute at the next
            # clock tick; no assumptions about proprietary syntax are made.
            rows.append((0.0, line))
    return rows


def program(paths: list[Path], limit: int) -> dict:
    rows: list[tuple[float, str]] = []
    for path in paths:
        rows.extend(parse(path, max(0, limit - len(rows))))
        if len(rows) >= limit:
            break
    if not rows:
        raise ValueError("no bounded log lines found")
    commands = []
    previous = 0.0
    for index, (stamp, text) in enumerate(rows):
        delay = max(0.0, stamp - previous)
        if delay:
            commands.append({"tag": "pause", "args": {"seconds": delay}})
        if text:
            commands.append({"tag": "text", "args": {"text": text, "source_index": index}})
        previous = max(previous, stamp)
    return {"schema": "keygen.story.v1", "blocks": [{"id": "private_log", "commands": commands}], "labels": {"start": 0}}


def self_test() -> None:
    with tempfile.TemporaryDirectory(dir="/tmp") as temp:
        source = Path(temp) / "bios.txt"
        source.write_text("0.0: first\n[1.5] second\nthird\n", encoding="utf-8")
        result = program([source], 8)
        assert result["schema"] == "keygen.story.v1"
        assert len(result["blocks"][0]["commands"]) == 4
        assert result["blocks"][0]["commands"][2]["tag"] == "text"
    print("private story-log importer self-test: ok")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--limit", type=int, default=512)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.paths or not args.output:
        parser.error("provide one or more log paths and --output")
    root = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    if root == output or root in output.parents:
        parser.error("refusing to write recovered text inside the repository")
    result = program([path.resolve() for path in args.paths], args.limit)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(output), "lines": len(result["blocks"][0]["commands"])}, sort_keys=True))


if __name__ == "__main__":
    main()
