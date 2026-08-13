#!/usr/bin/env python3
"""Fail-closed audit for private package route/scene/content closure.

This reads only a generated package under the operator's private output
directory. It verifies that routes point at renderable scene JSON, every scene
asset exists in the project manifest, and story route entries are declared.
"""
from __future__ import annotations
import argparse, json, tempfile
from pathlib import Path

def audit(package: Path) -> dict:
    project = json.loads((package / "project.json").read_text())
    routes = json.loads((package / "routes.json").read_text())
    assets = {a["id"] for a in project.get("assets", [])}
    scenes = {s["id"]: s for s in project.get("scenes", [])}
    errors = []
    for scene_id, scene in scenes.items():
        if not scene.get("id") or any(a not in assets for a in scene.get("asset_ids", [])):
            errors.append(f"scene asset reference invalid: {scene_id}")
        path = package / "scenes" / (scene_id.removeprefix("scene.") + ".json")
        if not path.is_file(): errors.append(f"scene document missing: {scene_id}")
    labels = set(project.get("story", {}).get("labels", []))
    route_ids = set()
    for route in routes.get("routes", []):
        route_ids.add(route.get("id"))
        scene_id, entry = route.get("scene"), route.get("story_entry")
        if scene_id not in scenes: errors.append(f"route scene missing: {route.get('id')}")
        if entry and entry not in labels: errors.append(f"route story label missing: {route.get('id')}")
    if routes.get("entry") and routes["entry"] not in scenes:
        errors.append("route entry scene missing")
    if errors: raise ValueError("; ".join(errors))
    return {"schema": "keygen.private.route_audit.v1", "routes": len(route_ids), "scenes": len(scenes), "assets": len(assets), "errors": []}

def main() -> None:
    p = argparse.ArgumentParser(description=__doc__); p.add_argument("package", nargs="?", type=Path); p.add_argument("--self-test", action="store_true"); a = p.parse_args()
    if a.self_test:
        with tempfile.TemporaryDirectory() as t:
            root = Path(t); (root / "scenes").mkdir()
            (root / "scenes" / "text.json").write_text(json.dumps({"schema":"keygen.scene.v1","id":"scene.text","asset_ids":["a"]}))
            (root / "project.json").write_text(json.dumps({"assets":[{"id":"a"}],"scenes":[{"id":"scene.text","asset_ids":["a"]}],"story":{"labels":["start"]}}))
            (root / "routes.json").write_text(json.dumps({"entry":"scene.text","routes":[{"id":"route.text","scene":"scene.text","story_entry":"start"}]}))
            assert audit(root)["errors"] == []
        print("package route audit self-test: ok"); return
    if not a.package: p.error("PACKAGE is required")
    print(json.dumps(audit(a.package), sort_keys=True))

if __name__ == "__main__":
    try: main()
    except (OSError, KeyError, ValueError, json.JSONDecodeError) as e: print(f"package route audit failed: {e}"); raise SystemExit(2)
