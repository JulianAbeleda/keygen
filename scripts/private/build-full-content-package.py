#!/usr/bin/env python3
"""Build a bounded, metadata-driven private KeyGen project package.

The package is deliberately written below ``/tmp``.  It reuses the operator's
ExportedProject bytes (copied by content hash), while the checked-in engine
only sees generic manifests.  Every catalog record is represented in a
project scene, and text records are combined into a deterministic story
program so the result is useful for reachability and host qualification.
"""
from __future__ import annotations

import argparse, hashlib, json, os, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXTRACT = ROOT / "scripts/private/extract-project-metadata.py"
IMPORT = ROOT / "scripts/private/import-story-log.py"

def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1024 * 1024): h.update(chunk)
    return h.hexdigest()

def private_output(path: Path) -> Path:
    path = path.resolve()
    tmp = Path("/tmp").resolve()
    if tmp != path and tmp not in path.parents:
        raise ValueError("output must be beneath /tmp")
    if path == ROOT or ROOT in path.parents:
        raise ValueError("refusing repository output")
    return path

def category(record: dict) -> str:
    kind, rel = record.get("kind"), record.get("source_path", "").lower()
    if kind == "audio": return "audio"
    if kind == "font": return "fonts"
    if kind == "image": return "sprites" if any(x in rel for x in ("sprite", "character", "portrait", "pose", "expression")) else "images"
    if kind == "text" and rel.endswith((".unity", ".scene", ".prefab")): return "scenes"
    return "text" if kind == "text" else "other"

def build(source: Path, output: Path, limit: int, max_bytes: int) -> dict:
    source, output = source.resolve(), private_output(output)
    if not source.is_dir(): raise ValueError("source must be an ExportedProject directory")
    output.mkdir(parents=True, exist_ok=True)
    metadata = output / "metadata.json"
    subprocess.run([sys.executable, str(EXTRACT), str(source), "--output", str(metadata), "--max-files", str(limit)], cwd=ROOT, check=True)
    data = json.loads(metadata.read_text(encoding="utf-8"))
    records = sorted(data["catalog"]["assets"], key=lambda r: r["logical_id"])
    if len(records) > limit: raise ValueError("record limit exceeded")
    if output.exists():
        for child in output.iterdir():
            if child.name != "metadata.json": shutil.rmtree(child) if child.is_dir() else child.unlink()
    assets_dir, scenes_dir = output / "assets", output / "scenes"
    assets_dir.mkdir(); scenes_dir.mkdir()
    assets, total = [], 0
    for record in records:
        src = (source / record["source_path"]).resolve()
        if source not in src.parents or not src.is_file(): raise ValueError(f"missing source: {record['source_path']}")
        actual = digest(src)
        if actual != record["source_sha256"]: raise ValueError(f"hash mismatch: {record['logical_id']}")
        total += src.stat().st_size
        if total > max_bytes: raise ValueError("byte limit exceeded")
        target = assets_dir / actual
        shutil.copyfile(src, target)
        assets.append({"id": record["logical_id"], "kind": record["kind"], "logical_path": f"assets/{actual}", "sha256": actual})
    grouped = {}
    for record in records: grouped.setdefault(category(record), []).append(record["logical_id"])
    for name, ids in sorted(grouped.items()):
        (scenes_dir / f"{name}.json").write_text(json.dumps({"schema":"keygen.scene.v1", "id":f"scene.{name}", "asset_ids":ids, "design_width":1920, "design_height":1080}, indent=2, sort_keys=True) + "\n")
    text_paths = [source / r["source_path"] for r in records if r["kind"] == "text"]
    story = output / "story.json"
    subprocess.run([sys.executable, str(IMPORT), *map(str, text_paths), "--output", str(story), "--limit", str(limit)], cwd=ROOT, check=True)
    labels = ["start"] + [f"category.{name}" for name in sorted(grouped)]
    project = {"schema":"keygen.project.v1", "project":{"id":"keygen.private.full-content", "display_name":"KeyGen private content package", "version":"0.1.0"}, "viewport":{"width":1920,"height":1080}, "assets":assets, "scenes":[{"id":f"scene.{name}","asset_ids":ids} for name,ids in sorted(grouped.items())], "story":{"entry":"start","labels":labels}, "persistence":{"namespace":"keygen.private.full-content","schema":"keygen.project.state.v1"}}
    (output / "project.json").write_text(json.dumps(project, indent=2, sort_keys=True) + "\n")
    routes = {"schema":"keygen.launcher.routes.v1", "entry":"scene.text", "routes":[{"id":f"route.{name}","scene":f"scene.{name}","asset_count":len(ids)} for name,ids in sorted(grouped.items())], "story":"story.json"}
    (output / "routes.json").write_text(json.dumps(routes, indent=2, sort_keys=True) + "\n")
    return {"output":str(output), "assets":len(assets), "bytes":total, "scenes":len(grouped), "story":str(story), "routes":str(output / "routes.json")}

def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--source", type=Path, required=False); p.add_argument("--output", type=Path, default=Path("/tmp/keygen-full-content")); p.add_argument("--limit", type=int, default=10000); p.add_argument("--max-bytes", type=int, default=20 * 1024**3); p.add_argument("--self-test", action="store_true")
    a = p.parse_args()
    if a.self_test:
        with tempfile.TemporaryDirectory(dir="/tmp") as t:
            src=Path(t)/"ExportedProject"; (src/"Assets").mkdir(parents=True); (src/"Assets"/"a.txt").write_text("hello\n")
            result=build(src, Path(t)/"out", 20, 100000); assert result["assets"] == 1 and Path(result["routes"]).is_file()
        print("full content package self-test: ok"); return
    if not a.source: p.error("--source is required")
    print(json.dumps(build(a.source, a.output, a.limit, a.max_bytes), sort_keys=True))

if __name__ == "__main__":
    try: main()
    except (OSError, ValueError, subprocess.CalledProcessError) as e: print(f"full content package failed: {e}", file=sys.stderr); raise SystemExit(2)
