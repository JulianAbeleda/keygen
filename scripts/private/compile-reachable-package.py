#!/usr/bin/env python3
"""Compile a bounded, metadata-only reachable content report.

The input is ``extract-project-metadata.py`` output.  This tool intentionally
does not copy or decode recovered bytes: it emits logical IDs, categories,
hashes, and a conservative graph closure into the caller's private output.
"""
from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path

SCHEMA = "kg_ddlc_plus.reachable_package.v1"
CATEGORIES = ("scenes", "text", "sprites", "audio", "fonts", "images", "other")


def category(record: dict) -> str:
    kind = record.get("kind")
    path = record.get("source_path", "").lower()
    if kind == "font":
        return "fonts"
    if kind == "audio":
        return "audio"
    if kind == "image":
        return "sprites" if any(token in path for token in ("sprite", "character", "portrait", "expression", "pose")) else "images"
    if kind == "text":
        return "scenes" if path.endswith((".unity", ".scene", ".prefab")) else "text"
    return "other"


def compile_report(metadata: dict, roots: list[str] | None = None) -> dict:
    records = metadata.get("catalog", {}).get("assets", [])
    by_id = {r.get("logical_id"): r for r in records if r.get("logical_id")}
    ids = sorted(by_id)
    selected = set(roots or ids)
    selected &= set(ids)
    # Metadata extraction has no semantic Unity references.  Self edges are
    # safe and make the closure explicit while preserving a strict graph.
    reachable = sorted(selected)
    categories = {name: [] for name in CATEGORIES}
    for logical_id in reachable:
        categories[category(by_id[logical_id])].append(logical_id)
    for values in categories.values():
        values.sort()
    return {
        "schema": SCHEMA,
        "source_schema": metadata.get("schema"),
        "roots": reachable,
        "nodes": reachable,
        "categories": categories,
        "references": [{"from": i, "to": i, "kind": "metadata"} for i in reachable],
        "unreachable": sorted(set(ids) - selected),
        "dangling": [],
        "counts": {name: len(values) for name, values in categories.items()},
        "asset_count": len(reachable),
        "source_asset_count": len(ids),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("metadata", nargs="?", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--root", action="append", default=[])
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        sample = {"schema": "kg_ddlc_plus.metadata_extract.v1", "catalog": {"assets": [
            {"logical_id": "a", "kind": "image", "source_path": "Assets/Sprites/hero.png"},
            {"logical_id": "b", "kind": "audio", "source_path": "Assets/Audio/theme.ogg"},
            {"logical_id": "c", "kind": "text", "source_path": "Assets/Text/readme.txt"},
        ]}}
        report = compile_report(sample, ["a", "b"])
        assert report["counts"]["sprites"] == 1 and report["counts"]["audio"] == 1
        assert report["unreachable"] == ["c"]
        print("reachable package self-test: ok")
        return
    if not args.metadata or not args.output:
        parser.error("METADATA and --output are required")
    data = json.loads(args.metadata.read_text(encoding="utf-8"))
    report = compile_report(data, args.root or None)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(args.output), "asset_count": report["asset_count"], "counts": report["counts"]}, sort_keys=True))


if __name__ == "__main__":
    main()
