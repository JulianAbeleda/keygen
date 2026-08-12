#!/usr/bin/env python3
"""Reject product identity/content tokens from reusable KeyGen crates."""

from pathlib import Path

FORBIDDEN = ("ddlc", "kg_ddlc", "team-salvato", "com.julian")
ROOTS = ("crates/engine", "crates/player")

def main() -> int:
    root = Path(__file__).resolve().parents[1]
    errors = []
    for relative in ROOTS:
        directory = root / relative
        for path in directory.rglob("*.rs"):
            text = path.read_text(encoding="utf-8").lower()
            for token in FORBIDDEN:
                if token in text:
                    errors.append(f"{path}: reusable crate contains product token {token!r}")
    if errors:
        print("\n".join(errors))
        return 1
    print("generic boundary content OK")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
