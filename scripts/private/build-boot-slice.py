#!/usr/bin/env python3
"""Build a bounded, private, playable boot slice from a local recovery.

The output is deliberately restricted to ``/tmp``.  It copies only the
operator-selected BIOS logo/font (when present), writes a generic
``keygen.project.v1`` manifest and a small scene, imports the BIOS log as a
generic story, then runs the generic validator and renderer.  No recovered
bytes or absolute source paths can enter the repository.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXTRACT = ROOT / "scripts/private/extract-project-metadata.py"
IMPORT = ROOT / "scripts/private/import-story-log.py"


def private_path(path: Path) -> Path:
    path = path.resolve()
    if Path("/tmp").resolve() not in (path, *path.parents):
        raise ValueError("output must be beneath /tmp")
    if path == ROOT or ROOT in path.parents:
        raise ValueError("refusing repository output")
    return path


def run(cmd: list[str]) -> None:
    subprocess.run(cmd, cwd=ROOT, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("/tmp/keygen-boot-slice"))
    parser.add_argument("--metadata", type=Path)
    parser.add_argument("--max-assets", type=int, default=4)
    args = parser.parse_args()
    source = args.source.resolve()
    output = private_path(args.output)
    if not source.is_dir():
        raise SystemExit("source must be an ExportedProject directory")
    output.mkdir(parents=True, exist_ok=True)
    metadata = args.metadata.resolve() if args.metadata else output / "metadata.json"
    if not args.metadata:
        run([sys.executable, str(EXTRACT), str(source), "--output", str(metadata), "--max-files", "10000"])
    data = json.loads(metadata.read_text(encoding="utf-8"))
    records = data.get("catalog", {}).get("assets", [])

    # Stable, logical selection: the known BIOS slice only.  Fallbacks use
    # suffixes rather than absolute paths or guessed content.
    wanted = {
        "Assets/TextAsset/bios.txt": "story",
        "Assets/Font/ModernDOS8x16.ttf": "font",
        "Assets/Texture2D/MES Logo bios 2.png": "logo",
    }
    chosen: dict[str, dict] = {}
    for record in records:
        rel = record.get("source_path", "")
        if rel in wanted:
            chosen[wanted[rel]] = record
    if "story" not in chosen:
        raise SystemExit("recovery does not contain Assets/TextAsset/bios.txt")
    if len(chosen) > args.max_assets:
        raise SystemExit("boot selection exceeds --max-assets")

    if output.exists():
        for child in output.iterdir():
            if child.name != "metadata.json":
                shutil.rmtree(child) if child.is_dir() else child.unlink()
    assets_dir = output / "assets"
    scenes_dir = output / "scenes"
    assets_dir.mkdir(exist_ok=True)
    scenes_dir.mkdir(exist_ok=True)
    assets = []
    paths: dict[str, str] = {}
    for role, record in sorted(chosen.items()):
        src = (source / record["source_path"]).resolve()
        if source not in src.parents or not src.is_file():
            raise SystemExit(f"missing selected source: {record['source_path']}")
        digest = hashlib.sha256(src.read_bytes()).hexdigest()
        if digest != record.get("source_sha256"):
            raise SystemExit(f"source hash mismatch: {record['logical_id']}")
        target = assets_dir / digest
        shutil.copyfile(src, target)
        paths[role] = f"../assets/{digest}"
        assets.append({"id": record["logical_id"], "kind": record["kind"],
                       "logical_path": f"assets/{digest}", "sha256": digest})

    story = output / "story.json"
    run([sys.executable, str(IMPORT), str(source / "Assets/TextAsset/bios.txt"), "--output", str(story), "--limit", "512"])
    story_data = json.loads(story.read_text(encoding="utf-8"))
    if story_data.get("schema") != "keygen.story.v1" or not story_data.get("blocks"):
        raise SystemExit("story smoke failed: empty keygen.story.v1")
    scene = {"schema": "keygen.scene.v1", "title": "KeyGen boot slice",
             "design_width": 1920, "design_height": 1080, "clear": [8, 10, 16, 255],
             "font_path": paths.get("font", ""), "layers": [], "particle_insertions": [],
             "menu_insertion": None, "menu": None, "text_layers": [{
                 "id": "boot-status", "text": "KEYGEN BOOT", "x": 64.0, "y": 64.0,
                 "font_size": 32.0, "color": [220, 230, 245, 255],
                 "outline": [0, 0, 0, 255], "outline_width": 1, "visible_at": 0.0,
                 "characters_per_second": None}], "particles": None, "fade": None}
    if "logo" in paths:
        scene["layers"].append({"id": "bios-logo", "path": paths["logo"], "x": 960.0,
                                "y": 420.0, "scale": 1.0, "anchor": "center", "alpha": 1.0,
                                "entrance": None, "motion": None})
    (scenes_dir / "boot.json").write_text(json.dumps(scene, indent=2) + "\n")
    project = {"schema": "keygen.project.v1",
               "project": {"id": "keygen.private.boot-slice", "display_name": "KeyGen private boot slice", "version": "0.1.0"},
               "viewport": {"width": 1920, "height": 1080}, "assets": assets,
               "scenes": [{"id": "scene.boot", "asset_ids": [a["id"] for a in assets]},
               ], "story": {"entry": "start", "labels": ["start"]},
               "persistence": {"namespace": "keygen.private.boot-slice", "schema": "keygen.project.state.v1"}}
    project_path = output / "project.json"
    project_path.write_text(json.dumps(project, indent=2) + "\n")
    render = output / "boot.png"
    run(["cargo", "run", "-q", "-p", "keygen", "--", "validate", str(project_path)])
    run(["cargo", "run", "-q", "-p", "keygen", "--", "render", str(project_path), "--scene", str(scenes_dir / "boot.json"), "--output", str(render), "--time", "0"])
    print(json.dumps({"output": str(output), "assets": len(assets), "story_commands": len(story_data["blocks"][0]["commands"]), "render": str(render)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"boot-slice failed: {error}", file=sys.stderr)
        raise SystemExit(2)
