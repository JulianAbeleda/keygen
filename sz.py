#!/usr/bin/env python3
"""Report and enforce KeyGen's owned production source-line budget."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


MAX_LINE_COUNT = 50_000
PRODUCTION_ROOTS = ("crates", "src", "platform")
PRODUCTION_FILES = ("build.rs",)
SOURCE_SUFFIXES = {
    ".c", ".cc", ".cpp", ".cs", ".go", ".h", ".hpp", ".m", ".mm",
    ".py", ".rs", ".sh", ".swift",
}
EXCLUDED_PARTS = {
    ".git", "benches", "docs", "examples", "fixtures", "generated",
    "target", "test", "tests", "third_party", "vendor",
}
TOKEN_RE = re.compile(
    r"[A-Za-z_][A-Za-z0-9_]*|\d+(?:\.\d+)?|==|!=|<=|>=|->|=>|::|&&|\|\||[^\s]"
)


@dataclass(frozen=True)
class FileStats:
    path: str
    lines: int
    tokens: int

    @property
    def tokens_per_line(self) -> float:
        return self.tokens / self.lines if self.lines else 0.0


def is_production_file(path: Path, root: Path) -> bool:
    relative = path.relative_to(root)
    if any(part in EXCLUDED_PARTS for part in relative.parts):
        return False
    return path.name in PRODUCTION_FILES or path.suffix.lower() in SOURCE_SUFFIXES


def production_files(root: Path) -> list[Path]:
    paths: set[Path] = set()
    for directory in PRODUCTION_ROOTS:
        candidate = root / directory
        if candidate.is_dir():
            paths.update(
                path for path in candidate.rglob("*")
                if path.is_file() and is_production_file(path, root)
            )
    for filename in PRODUCTION_FILES:
        candidate = root / filename
        if candidate.is_file():
            paths.add(candidate)
    return sorted(paths)


def count_file(path: Path, root: Path) -> FileStats:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = [line for line in text.splitlines() if line.strip()]
    return FileStats(
        path.relative_to(root).as_posix(),
        len(lines),
        sum(len(TOKEN_RE.findall(line)) for line in lines),
    )


def group_name(path: str) -> str:
    parts = Path(path).parts
    return "/".join(parts[:2]) if len(parts) >= 2 and parts[0] == "crates" else parts[0]


def report(root: Path) -> dict[str, object]:
    files = [count_file(path, root) for path in production_files(root)]
    groups: dict[str, dict[str, int]] = defaultdict(
        lambda: {"files": 0, "lines": 0, "tokens": 0}
    )
    for item in files:
        group = groups[group_name(item.path)]
        group["files"] += 1
        group["lines"] += item.lines
        group["tokens"] += item.tokens
    total = sum(item.lines for item in files)
    return {
        "limit": MAX_LINE_COUNT,
        "lines": total,
        "remaining": MAX_LINE_COUNT - total,
        "within_budget": total <= MAX_LINE_COUNT,
        "groups": dict(sorted(groups.items())),
        "files": [
            {
                "path": item.path,
                "lines": item.lines,
                "tokens_per_line": round(item.tokens_per_line, 1),
            }
            for item in sorted(files, key=lambda value: (-value.lines, value.path))
        ],
    }


def print_text(data: dict[str, object], top: int) -> None:
    files = data["files"]
    groups = data["groups"]
    assert isinstance(files, list)
    assert isinstance(groups, dict)
    print("KeyGen production size")
    print("\nLargest files")
    if not files:
        print("  (no production source files yet)")
    for item in files[:top]:
        print(f"  {item['lines']:6d}  {item['tokens_per_line']:5.1f} tok/line  {item['path']}")
    print("\nSubsystems")
    if not groups:
        print("  (no production subsystems yet)")
    for name, item in groups.items():
        density = item["tokens"] / item["lines"] if item["lines"] else 0.0
        print(f"  {item['lines']:6d}  {density:5.1f} tok/line  {name} ({item['files']} files)")
    print(f"\nproduction lines: {data['lines']}")
    print(f"hard limit:       {data['limit']}")
    print(f"remaining:        {data['remaining']}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--top", type=int, default=20)
    args = parser.parse_args()
    data = report(args.root.resolve())
    if args.json:
        print(json.dumps(data, indent=2, sort_keys=True))
    else:
        print_text(data, max(args.top, 0))
    if not data["within_budget"]:
        print(f"ERROR: production source exceeds {MAX_LINE_COUNT} lines", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
