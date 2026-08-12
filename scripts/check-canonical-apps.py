#!/usr/bin/env python3
"""Reject stale or duplicate local macOS app bundles.

The product has one canonical bundle name. This check is intentionally
non-destructive: it only inspects the repository's generated distribution
directory and passes when no local distribution has been built yet.
"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist" / "macos"
CANONICAL = "kg_ddlc_plus.app"


def main() -> int:
    if not DIST.exists():
        print("canonical app check: OK (no local macOS distribution)")
        return 0
    bundles = sorted(path.name for path in DIST.iterdir() if path.name.endswith(".app"))
    if bundles != [CANONICAL]:
        print(
            "canonical app check: expected only "
            f"{CANONICAL!r}, found {bundles!r}",
            file=sys.stderr,
        )
        return 1
    app = DIST / CANONICAL
    executable = app / "Contents" / "MacOS" / "kg_ddlc_plus"
    plist = app / "Contents" / "Info.plist"
    if not executable.is_file() or not plist.is_file():
        print("canonical app check: incomplete canonical bundle", file=sys.stderr)
        return 1
    print("canonical app check: OK (one complete canonical bundle)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
