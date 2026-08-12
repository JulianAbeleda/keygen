#!/usr/bin/env python3
"""Check that reusable KeyGen layers do not depend on product crates."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


RULES = {
    "crates/engine": ("kg_ddlc_plus", "keygen_player", "keygen-player"),
    "crates/player": ("kg_ddlc_plus", "kg-ddlc-plus"),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    errors: list[str] = []
    for relative, forbidden in RULES.items():
        directory = args.root / relative
        if not directory.is_dir():
            continue
        for path in directory.rglob("*.rs"):
            text = path.read_text(encoding="utf-8")
            for dependency in forbidden:
                if re.search(rf"(?:use|extern crate|path\\s*=).*{re.escape(dependency)}", text):
                    errors.append(f"{path}: reusable layer imports {dependency}")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("module boundaries OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
