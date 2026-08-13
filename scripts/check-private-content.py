#!/usr/bin/env python3
"""Reject private recovery content and host-specific paths.

The default mode scans only Git-tracked files, so ignored local recovery trees
are never read.  ``--path`` is intended for a temporary synthetic fixture or
an explicitly selected staging directory.  It does not recursively inspect a
player's installation unless the operator opts into that path.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path


MAX_BYTES = 2 * 1024 * 1024
TEXT_SUFFIXES = {".md", ".py", ".sh", ".toml", ".json", ".yaml", ".yml", ".rs", ".txt"}

# These are deliberately narrow indicators of a recovered installation, not
# broad words such as "asset", "source", or a product name used in docs.
PRIVATE_PATTERNS = (
    (re.compile(r"(?:^|[/\\])(?:Users|home|Volumes)[/\\][^\n\r]*", re.I), "host absolute path"),
    (re.compile(r"(?:steamapps[/\\]common|assetripper-build|ExportedProject)[/\\]", re.I), "recovery path"),
    (re.compile(r"(?:^|[/\\])(?:globalgamemanagers|level\d+|sharedassets\d+\.assets|resources\.assets)(?:$|[/\\])", re.I), "Unity payload path"),
    (re.compile(r"(?:Doki Doki Literature Club Plus|DDLC Plus)[/\\].*\.(?:png|jpg|wav|ogg|ttf|asset|bundle)$", re.I), "proprietary payload reference"),
)
HASH_RE = re.compile(r"\b[0-9a-f]{64}\b", re.I)


def tracked_files(root: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        stdout=subprocess.PIPE,
    )
    return [root / item for item in result.stdout.decode().split("\0") if item]


def scan_file(path: Path, *, relative: str) -> list[str]:
    if path.suffix.lower() not in TEXT_SUFFIXES or path.stat().st_size > MAX_BYTES:
        return []
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return []
    findings: list[str] = []
    for number, line in enumerate(text.splitlines(), 1):
        for pattern, label in PRIVATE_PATTERNS:
            if pattern.search(line):
                findings.append(f"{relative}:{number}: {label}")
        # Hashes are allowed in Cargo.lock and public dependency manifests.
        # In other files require a nearby source/payload cue to avoid treating
        # ordinary content-addressed documentation as proprietary content.
        if path.name != "Cargo.lock" and HASH_RE.search(line) and re.search(
            r"(?:private|recovery|source|payload|asset|proprietary)", line, re.I
        ):
            findings.append(f"{relative}:{number}: private-looking content hash")
    return findings


def scan(root: Path, paths: list[Path] | None = None) -> list[str]:
    files = paths if paths is not None else tracked_files(root)
    findings: list[str] = []
    for path in files:
        if not path.is_file():
            continue
        try:
            relative = str(path.relative_to(root))
        except ValueError:
            relative = str(path)
        findings.extend(scan_file(path, relative=relative))
    return findings


def self_test(root: Path) -> int:
    with tempfile.TemporaryDirectory(prefix="keygen-private-scan-") as directory:
        fixture = Path(directory) / "synthetic.txt"
        fixture.write_text(
            "source=" + chr(47) + "Users/example/Steam/" + "/".join(("steamapps", "common", "Doki Doki Literature Club Plus", "ExportedProject")) + "\n"
            "private asset hash: " + "a" * 64 + "\n",
            encoding="utf-8",
        )
        if not scan(root, [fixture]):
            print("self-test failed: synthetic private fixture was accepted", file=sys.stderr)
            return 1
        clean = Path(directory) / "clean.md"
        clean.write_text("A public synthetic fixture with a normal example.\n", encoding="utf-8")
        if scan(root, [clean]):
            print("self-test failed: clean fixture was rejected", file=sys.stderr)
            return 1
    print("private-content scanner self-test passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--path", action="append", type=Path, help="explicit file or directory to scan")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    if args.self_test:
        return self_test(root)
    if args.path:
        paths: list[Path] = []
        for selected in args.path:
            selected = selected.resolve()
            if selected.is_dir():
                paths.extend(p for p in selected.rglob("*") if p.is_file())
            elif selected.is_file():
                paths.append(selected)
        findings = scan(root, paths)
    else:
        findings = scan(root)
    if findings:
        print("private-content scan failed:", file=sys.stderr)
        print("\n".join(findings), file=sys.stderr)
        return 1
    print("private-content scan passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
