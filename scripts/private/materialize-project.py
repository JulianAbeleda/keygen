#!/usr/bin/env python3
"""Materialize a bounded private KeyGen package from metadata and source bytes.

This is deliberately an operator-only bridge.  It requires an explicit
ExportedProject and writes only beneath /tmp (or another explicitly approved
temporary directory), never ``local/`` or any tracked repository directory.
Selection is bounded by logical IDs, shell-style globs, and a hard limit.
"""
from __future__ import annotations
import argparse, fnmatch, hashlib, json, shutil, tempfile
from pathlib import Path

PROJECT_SCHEMA = "keygen.project.v1"
META_SCHEMA = "kg_ddlc_plus.metadata_extract.v1"

def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1024 * 1024): h.update(chunk)
    return h.hexdigest()

def safe_output(path: Path) -> Path:
    resolved = path.resolve()
    temp_root = Path("/tmp").resolve()
    if not (resolved == temp_root or temp_root in resolved.parents):
        raise ValueError("output must be beneath /tmp; private materialization cannot write the repository")
    if resolved.name in {"local", "keygen"}:
        raise ValueError("refusing unsafe output directory")
    return resolved

def materialize(metadata_path: Path, source: Path, output: Path, ids: list[str], globs: list[str], limit: int) -> dict:
    source, output = source.resolve(), safe_output(output)
    if not source.is_dir(): raise ValueError("source must be an ExportedProject directory")
    metadata = json.loads(metadata_path.read_text())
    if metadata.get("schema") not in (None, META_SCHEMA) or metadata.get("catalog", {}).get("schema") != "keygen.assets.v1":
        raise ValueError("unsupported metadata schema")
    records = metadata.get("catalog", {}).get("assets", [])
    if not isinstance(records, list): raise ValueError("metadata catalog assets must be a list")
    chosen = []
    for record in records:
        logical = record.get("logical_id", "")
        matches_id = logical in ids
        matches_glob = any(fnmatch.fnmatchcase(logical, pattern) for pattern in globs)
        if (not ids and not globs) or matches_id or matches_glob: chosen.append(record)
    # IDs and globs are unioned, while preserving metadata order.
    unique = {r["logical_id"]: r for r in chosen}
    chosen = [unique[k] for k in sorted(unique)]
    if len(chosen) > limit: raise ValueError(f"selection exceeds limit ({len(chosen)} > {limit})")
    if not chosen: raise ValueError("selection is empty; provide matching --id/--glob or metadata with assets")
    if output.exists(): shutil.rmtree(output)
    (output / "assets").mkdir(parents=True)
    assets, selected_ids = [], set()
    for record in chosen:
        rel = record.get("source_path")
        if not rel: raise ValueError(f"asset {record.get('logical_id')} has no relative source_path; regenerate metadata")
        src = (source / rel).resolve()
        if source not in src.parents: raise ValueError("source_path escapes ExportedProject")
        if not src.is_file(): raise ValueError(f"missing source asset: {rel}")
        actual = digest(src)
        if actual != record.get("source_sha256"): raise ValueError(f"source hash mismatch: {record['logical_id']}")
        target_rel = f"assets/{actual}"
        shutil.copyfile(src, output / target_rel)
        assets.append({"id": record["logical_id"], "kind": record["kind"], "logical_path": target_rel, "sha256": actual})
        selected_ids.add(record["logical_id"])
    scenes = []
    for scene in metadata.get("content", {}).get("roots", []):
        refs = [r.get("to") for r in metadata.get("content", {}).get("references", []) if r.get("from") == scene and r.get("to") in selected_ids]
        scenes.append({"id": scene, "asset_ids": sorted(set(refs))})
    project = {"schema": PROJECT_SCHEMA, "project": metadata["identity"], "viewport": metadata["viewport"],
               "assets": sorted(assets, key=lambda a: a["id"]), "scenes": scenes, "story": None,
               "persistence": metadata.get("persistence", {"namespace": "keygen.project", "schema": "keygen.project.state.v1"})}
    (output / "project.json").write_text(json.dumps(project, indent=2, sort_keys=True) + "\n")
    return {"output": str(output), "assets": len(assets), "bytes": sum((output / a["logical_path"]).stat().st_size for a in assets)}

def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--metadata", type=Path); p.add_argument("--source", type=Path); p.add_argument("--output", type=Path)
    p.add_argument("--id", action="append", default=[]); p.add_argument("--glob", action="append", default=[])
    p.add_argument("--limit", type=int, default=256); p.add_argument("--self-test", action="store_true")
    a = p.parse_args()
    if a.self_test:
        with tempfile.TemporaryDirectory(dir="/tmp") as t:
            root, out = Path(t) / "ExportedProject", Path(t) / "pkg"; (root / "Assets").mkdir(parents=True)
            f = root / "Assets" / "hero.txt"; f.write_text("hero\n"); d = digest(f)
            meta = Path(t) / "meta.json"; meta.write_text(json.dumps({"schema": META_SCHEMA, "identity":{"id":"sample","display_name":"Sample","version":"0.1"},"viewport":{"width":1,"height":1},"persistence":{},"catalog":{"schema":"keygen.assets.v1","assets":[{"logical_id":"hero","kind":"text","source_sha256":d,"source_path":"Assets/hero.txt"}]},"content":{"roots":["scene"],"references":[{"from":"scene","to":"hero"}]}}))
            result = materialize(meta, root, out, [], [], 4); assert result["assets"] == 1 and json.loads((out / "project.json").read_text())["schema"] == PROJECT_SCHEMA
        print("private materializer self-test: ok"); return
    if not (a.metadata and a.source and a.output): p.error("--metadata, --source, and --output are required")
    print(json.dumps(materialize(a.metadata, a.source, a.output, a.id, a.glob, a.limit), sort_keys=True))

if __name__ == "__main__": main()
