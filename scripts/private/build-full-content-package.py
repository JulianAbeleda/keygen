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

def find_asset(records: list[dict], assets: list[dict], needle: str, kind: str = "image") -> dict | None:
    for record, asset in zip(records, assets):
        if record.get("kind") == kind and needle in record.get("source_path", "").lower():
            return asset
    return None

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
    # Category scenes are initially declared below; each receives the same
    # renderable boot composition with a category-specific title/status so a
    # route can actually load and display it.  Asset membership remains in the
    # project manifest and is audited independently.
    # A packaged application needs one concrete, renderable boot scene.  Keep
    # this generic: prefer the recovered DOS font/logo when present, but never
    # assume a title-specific asset exists.  The scene itself references only
    # copied package-relative bytes.
    # The recovered start-menu labels reference Vera SDF. Its source TTF is
    # not present in the export, so use the recovered neutral sans fallback;
    # never fall through to one of the unrelated poem handwriting fonts.
    font = next((candidate for candidate in (
        find_asset(records, assets, "liberationsans.ttf", "font"),
        find_asset(records, assets, "hkgrotesk-medium.otf", "font"),
    ) if candidate is not None), None)
    desktop_font = find_asset(records, assets, "hkgrotesk-medium.otf", "font") or font
    font = font or next((a for a, r in zip(assets, records) if r["kind"] == "font"), None)
    logo = find_asset(records, assets, "ddlc logo") or find_asset(records, assets, "logo")
    wallpaper = find_asset(records, assets, "gallery_default_wallpaper")
    start_panel = find_asset(records, assets, "start menu background")
    launcher_atlas = find_asset(records, assets, "sactx-2048x4096")
    drop_shadow = find_asset(records, assets, "uidropshadow.png")
    heart_glow = find_asset(records, assets, "heart highlight gradient.png")
    heart_icon = find_asset(records, assets, "heart icon white.png")
    ddlc_icon = find_asset(records, assets, "ddlc icon.png") or find_asset(records, assets, "ddlc icon")
    ddlc_icon_focused = find_asset(records, assets, "ddlc icon highlight.png")
    side_icon = find_asset(records, assets, "side stories icon.png") or find_asset(records, assets, "side stories icon")
    side_icon_focused = find_asset(records, assets, "side stories icon highlight.png")
    files_icon = find_asset(records, assets, "files icon_0.png") or find_asset(records, assets, "files icon.png")
    files_icon_focused = find_asset(records, assets, "file icons highlight.png")
    mail_icon = find_asset(records, assets, "mail icon_0.png") or find_asset(records, assets, "mail icon.png")
    mail_icon_focused = find_asset(records, assets, "mail icon highlight.png")
    photos_icon = find_asset(records, assets, "photos icon.png") or find_asset(records, assets, "photos icon")
    photos_icon_focused = find_asset(records, assets, "photo icon highlight.png")
    music_icon = find_asset(records, assets, "music icon_0.png") or find_asset(records, assets, "music icon.png")
    music_icon_focused = find_asset(records, assets, "music icon highlight.png")
    settings_icon = find_asset(records, assets, "settings icon_0.png") or find_asset(records, assets, "settings icon.png")
    settings_icon_focused = find_asset(records, assets, "settings icon highlight.png")
    if font is None:
        font = next((a for a in assets if a["kind"] == "font"), None)
    menu_entries = [
        {"id": "route.ddlc", "label": "DDLC", "enabled": True},
        {"id": "route.side_stories", "label": "Side Stories", "enabled": True},
        {"id": "route.files", "label": "Files", "enabled": True},
        {"id": "route.mail", "label": "Mail", "enabled": True},
        {"id": "route.pictures", "label": "Pictures", "enabled": True},
        {"id": "route.music", "label": "Music", "enabled": True},
        {"id": "route.settings", "label": "Settings", "enabled": True},
        {"id": "exit", "label": "Quit", "enabled": True},
    ]
    boot = {
        "schema": "keygen.scene.v1", "title": "DDLC Plus Launcher", "design_width": 1920,
        "design_height": 1080, "borderless": True,
        # Rendering remains the recovered 1920x1080; the macOS host discovers
        # the active display's point size and letterboxes instead of cropping.
        "fit_window_to_display": True, "immersive_system_ui": True,
        "clear": [0, 0, 0, 255],
        "font_path": f"../{font['logical_path']}" if font else "",
        "layers": [], "particle_insertions": [], "menu_insertion": 11,
        # The Unity launcher uses a 436x633 lower-left panel.  Its eight
        # 436x73 rows are laid out from the panel's top edge in reference
        # coordinates; keeping those coordinates here makes the native
        # compositor match the recovered prefab instead of a generic menu.
        "menu": {"x": 116.0, "y": 457.0, "width": 300.0, "row_height": 73.0,
        "spacing": 0.0, "font_size": 32.0, "outline_width": 1,
        "color": [123, 0, 102, 255], "focused_color": [255, 255, 255, 255],
        "outline": [255, 255, 255, 0],
        "focused_outline": [255, 255, 255, 0], "entries": menu_entries},
        "text_layers": [], "particles": None, "fade": None,
    }
    if wallpaper:
        boot["layers"].append({"id": "launcher-wallpaper", "path": f"../{wallpaper['logical_path']}",
                                "x": 960.0, "y": 540.0, "scale": 1.0, "anchor": "center",
                                "alpha": 1.0, "entrance": None, "motion": None,
                                "source_rect": None, "visible_when_focused": None})
    if drop_shadow:
        boot["layers"].append({"id": "start-menu-shadow", "path": f"../{drop_shadow['logical_path']}",
                                "x": -30.0, "y": 437.0, "scale": 1.0, "anchor": "top_left",
                                "alpha": 1.0, "entrance": None, "motion": None,
                                "source_rect": [27, 27, 456, 456], "visible_when_focused": None,
                                "nine_slice": {"left": 37, "top": 37, "right": 35, "bottom": 35,
                                               "width": 496.0, "height": 673.0}})
    if start_panel:
        boot["layers"].append({"id": "start-menu-panel", "path": f"../{start_panel['logical_path']}",
                                "x": 218.0, "y": 763.5, "scale": 0.5, "anchor": "center",
                                "alpha": 1.0, "entrance": None, "motion": None,
                                "source_rect": None, "visible_when_focused": None})
    if launcher_atlas:
        # Unity's source rect is bottom-origin. The compositor's PNG crop is
        # top-origin, hence 4096 - (1503 + 145) = 2448.
        for index in range(8):
            boot["layers"].append({"id": f"row-highlight-{index}",
                                    "path": f"../{launcher_atlas['logical_path']}",
                                    "x": 0.0, "y": 457.0 + index * 73.0, "scale": 0.50115,
                                    "anchor": "top_left", "alpha": 1.0,
                                    "entrance": None, "motion": None,
                                    "source_rect": [0, 2448, 870, 145],
                                    "visible_when_focused": index})
    # These source sprites are 85px UI icons and are displayed at native
    # launcher scale, one per visible application row.  The panel already
    # supplies the pink/cream surface behind them.
    icons = ((ddlc_icon, ddlc_icon_focused, "ddlc-icon", 0),
             (side_icon, side_icon_focused, "side-icon", 1),
             (files_icon, files_icon_focused, "files-icon", 2),
             (mail_icon, mail_icon_focused, "mail-icon", 3),
             (photos_icon, photos_icon_focused, "photos-icon", 4),
             (music_icon, music_icon_focused, "music-icon", 5),
             (settings_icon, settings_icon_focused, "settings-icon", 6))
    for asset, focused_asset, ident, index in icons:
        y = 493.5 + index * 73.0
        if asset:
            boot["layers"].append({"id": ident, "path": f"../{asset['logical_path']}",
                                    "x": 36.0, "y": y, "scale": 0.5, "anchor": "center",
                                    "alpha": 1.0, "entrance": None, "motion": None,
                                    "source_rect": None, "visible_when_focused": None})
        if focused_asset:
            boot["layers"].append({"id": f"{ident}-focused", "path": f"../{focused_asset['logical_path']}",
                                    "x": 36.0, "y": y, "scale": 0.5, "anchor": "center",
                                    "alpha": 1.0, "entrance": None, "motion": None,
                                    "source_rect": None, "visible_when_focused": index})
    # Desktop taskbar is an atlas crop in the same recovered UI texture.
    if launcher_atlas:
        boot["layers"].append({"id": "taskbar", "path": f"../{launcher_atlas['logical_path']}",
                                "x": 0.0, "y": 1020.0, "scale": 1.0, "anchor": "top_left",
                                "alpha": 1.0, "entrance": None, "motion": None,
                                "source_rect": [0, 2235, 1920, 60], "visible_when_focused": None})
    for asset, ident, x, y, scale in ((heart_glow, "start-heart-glow", 75.5, 1050.0, 0.5),
                                      (heart_icon, "start-heart", 75.5, 1050.0, 0.5)):
        if asset:
            boot["layers"].append({"id": ident, "path": f"../{asset['logical_path']}",
                                    "x": x, "y": y, "scale": scale, "anchor": "center",
                                    "alpha": 1.0, "entrance": None, "motion": None,
                                    "source_rect": None, "visible_when_focused": None})
    boot["text_layers"].append({"id": "launcher-clock", "text": "14:08", "x": 1614.0,
        "y": 1032.0, "font_size": 30.0, "color": [255, 255, 255, 255],
        "outline": [0, 0, 0, 0], "outline_width": 0, "visible_at": 0.0,
        "characters_per_second": None, "system_clock_24h": True,
        "font_path": f"../{desktop_font['logical_path']}" if desktop_font else None})
    (scenes_dir / "boot.json").write_text(json.dumps(boot, indent=2, sort_keys=True) + "\n")
    for name in sorted(grouped):
        category_scene = dict(boot)
        category_scene["title"] = f"KeyGen — {name.title()}"
        category_scene["menu"] = None
        category_scene["menu_insertion"] = None
        category_scene["text_layers"] = [{"id": "category-status", "text": f"KEYGEN / {name.upper()}",
            "x": 64.0, "y": 64.0, "font_size": 32.0, "color": [220, 230, 245, 255],
            "outline": [0, 0, 0, 255], "outline_width": 1, "visible_at": 0.0,
            "characters_per_second": None, "system_clock_24h": False, "font_path": None}]
        (scenes_dir / f"{name}.json").write_text(json.dumps(category_scene, indent=2, sort_keys=True) + "\n")
    text_paths = [source / r["source_path"] for r in records if r["kind"] == "text"]
    story = output / "story.json"
    subprocess.run([sys.executable, str(IMPORT), *map(str, text_paths), "--output", str(story), "--limit", str(limit)], cwd=ROOT, check=True)
    labels = ["start"] + [f"category.{name}" for name in sorted(grouped)]
    scenes = [{"id":f"scene.{name}","asset_ids":ids} for name,ids in sorted(grouped.items())]
    routes = [{"id":f"route.{name}","scene":f"scene.{name}","story_entry":None} for name in sorted(grouped)]
    # Preserve the recovered launcher semantics when the corresponding
    # categories exist, while retaining every generated category route.
    aliases = {
        "route.ddlc": "images",
        "route.side_stories": "sprites" if "sprites" in grouped else "images",
        "route.files": "text",
        "route.mail": "images",
        "route.pictures": "images",
        "route.music": "audio" if "audio" in grouped else "images",
        "route.settings": "fonts" if "fonts" in grouped else "images",
    }
    for route_id, category_name in aliases.items():
        if category_name in grouped:
            routes.append({"id": route_id, "scene": f"scene.{category_name}", "story_entry": None})
    project = {"schema":"keygen.project.v1", "project":{"id":"kg-ddlc-plus.private", "display_name":"KG DDLC Plus", "version":"0.1.0"}, "viewport":{"width":1920,"height":1080}, "assets":assets, "scenes":scenes, "routes":routes, "story":{"entry":"start","labels":labels}, "persistence":{"namespace":"kg-ddlc-plus.private","schema":"keygen.project.state.v1"}}
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
