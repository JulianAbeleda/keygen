#!/usr/bin/env python3
"""Fail-closed checks for a private kg_ddlc_plus asset catalog.

The checker accepts only metadata; it never requires or reads proprietary
source files. It is intended for package/build gates and synthetic CI tests.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

HASH = re.compile(r"^[0-9a-fA-F]{64}$")
SCHEMA = "keygen.assets.v1"
MODES = {"copy", "translate", "reimplement"}


def fail(message: str) -> int:
    print(f"asset reuse check: {message}", file=sys.stderr)
    return 1


def check(path: Path, root: Path | None) -> int:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return fail(f"cannot read catalog: {exc}")
    if document.get("schema") != SCHEMA:
        return fail("unsupported or missing catalog schema")
    if not document.get("importer_version"):
        return fail("missing importer version")
    blobs = document.get("blobs")
    assets = document.get("assets")
    if not isinstance(blobs, list) or not isinstance(assets, list):
        return fail("catalog requires blobs and assets arrays")
    if len(set(blobs)) != len(blobs):
        return fail("duplicate blob declaration")
    blob_set = set(blobs)
    ids: set[str] = set()
    for asset in assets:
        required = {"logical_id", "kind", "source_sha256", "output_sha256", "import_mode", "importer_version", "blob"}
        missing = required - asset.keys()
        if missing:
            return fail(f"asset missing fields: {sorted(missing)}")
        logical_id = asset["logical_id"]
        if not isinstance(logical_id, str) or not logical_id or logical_id in ids:
            return fail(f"duplicate or empty logical id: {logical_id!r}")
        ids.add(logical_id)
        if asset["import_mode"] not in MODES:
            return fail(f"forbidden import mode for {logical_id}")
        for field in ("source_sha256", "output_sha256"):
            if not isinstance(asset[field], str) or not HASH.fullmatch(asset[field]):
                return fail(f"invalid {field} for {logical_id}")
        blob = asset["blob"]
        if not isinstance(blob, str) or not blob or blob.startswith(("/", "\\")) or ".." in Path(blob).parts:
            return fail(f"unsafe blob path for {logical_id}")
        if blob not in blob_set:
            return fail(f"undeclared blob for {logical_id}: {blob}")
        if not asset["kind"] or not asset["importer_version"]:
            return fail(f"incomplete provenance for {logical_id}")
        image = asset.get("image")
        if image is not None:
            if not all(isinstance(image.get(k), int) and image[k] > 0 for k in ("width", "height")):
                return fail(f"invalid image dimensions for {logical_id}")
            if not isinstance(image.get("pixel_sha256"), str) or not HASH.fullmatch(image["pixel_sha256"]):
                return fail(f"invalid decoded pixel hash for {logical_id}")
    if root is not None:
        for blob in blobs:
            candidate = root / blob
            if not candidate.is_file():
                return fail(f"declared blob is missing: {blob}")
            expected = Path(blob).stem
            if HASH.fullmatch(expected) and hashlib.sha256(candidate.read_bytes()).hexdigest() != expected:
                return fail(f"blob hash mismatch: {blob}")
    print(f"asset reuse check: OK ({len(assets)} assets, {len(blobs)} blobs)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("catalog", type=Path, help="keygen.assets.v1 JSON catalog")
    parser.add_argument("--root", type=Path, help="package root for blob existence/hash checks")
    args = parser.parse_args()
    return check(args.catalog, args.root)


if __name__ == "__main__":
    raise SystemExit(main())
