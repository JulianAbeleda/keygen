#!/usr/bin/env python3
"""Bounded qualification for a generated KeyGen package.

All outputs remain below /tmp. The private source is optional: without it the
script still runs structural/self-test checks, while an available recovery is
fully extracted, audited, rendered, story-loaded, and packaged.
"""
from __future__ import annotations
import argparse, json, os, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE = Path.home() / "ddlc-architecture-explorer/unpacked/assetripper-build-10766092/ExportedProject"

def run(args: list[str]) -> str:
    return subprocess.run(args, cwd=ROOT, check=True, text=True, capture_output=True).stdout

def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    p.add_argument("--output", type=Path)
    a = p.parse_args()
    out = a.output or Path(tempfile.mkdtemp(prefix="keygen-e2e-", dir="/tmp"))
    out = out.resolve()
    if Path("/tmp").resolve() not in (out, *out.parents):
        raise SystemExit("output must be beneath /tmp")
    if not a.source.is_dir():
        run([sys.executable, "scripts/private/audit-package-routes.py", "--self-test"])
        print(json.dumps({"status": "skipped", "reason": "private source not present"}))
        return 0
    run([sys.executable, "scripts/private/build-full-content-package.py", "--source", str(a.source), "--output", str(out), "--limit", "10000", "--max-bytes", str(1024**3)])
    run([sys.executable, "scripts/private/audit-package-routes.py", str(out)])
    project = out / "project.json"
    run(["cargo", "run", "-q", "-p", "keygen", "--", "validate", str(project)])
    run(["cargo", "run", "-q", "-p", "keygen", "--", "story", str(project)])
    trace = run(["cargo", "run", "-q", "-p", "keygen", "--", "trace", str(project)])
    for marker in ("trace: Launcher", "trace: App", "trace: Closed"):
        if marker not in trace:
            raise ValueError(f"missing qualification trace marker: {marker}")
    run(["cargo", "run", "-q", "-p", "keygen", "--", "render", str(project), "--scene", str(out / "scenes/boot.json"), "--output", str(out / "boot.png"), "--time", "0"])
    if os.uname().sysname == "Darwin" and os.uname().machine == "arm64":
        run(["cargo", "build", "-q", "-p", "keygen", "--release"])
        dist = out / "dist"
        run(["scripts/package-keygen-macos.sh", "--binary", "target/release/keygen", "--target", "kg_ddlc_plus", "--display-name", "KG DDLC Plus", "--bundle-id", "com.julian.kg-ddlc-plus", "--resources", str(out), "--out", str(dist)])
        run(["scripts/smoke-keygen-app.sh", str(dist / "kg_ddlc_plus.app"), "kg_ddlc_plus", "com.julian.kg-ddlc-plus"])
    print(json.dumps({"status": "passed", "package": str(out), "boot": str(out / "boot.png")}))
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, subprocess.CalledProcessError, ValueError) as e:
        print(f"full package qualification failed: {e}", file=sys.stderr)
        raise SystemExit(2)
