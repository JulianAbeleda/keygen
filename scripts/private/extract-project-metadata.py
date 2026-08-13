#!/usr/bin/env python3
"""Extract bounded, metadata-only AssetRipper project information.

This operator tool reads an explicit local ExportedProject, hashes files in
place, and writes JSON that ``kg-ddlc-plus compile-project`` accepts.  It never
copies source bytes; catalog blob paths are metadata-only placeholders.  The
output is intended for a private working directory, not Git.
"""
from __future__ import annotations
import argparse, hashlib, json, os, tempfile
from pathlib import Path

SCHEMA = "kg_ddlc_plus.metadata_extract.v1"
HEX0 = "0" * 64
EXTENSIONS = {
    ".png": "image", ".jpg": "image", ".jpeg": "image", ".tga": "image",
    ".wav": "audio", ".ogg": "audio", ".mp3": "audio", ".m4a": "audio",
    ".ttf": "font", ".otf": "font", ".txt": "text", ".json": "text",
    ".csv": "text", ".xml": "text", ".asset": "text", ".prefab": "text",
    ".unity": "text", ".scene": "text",
}

def sha256(path: Path) -> tuple[str, int]:
    digest, size = hashlib.sha256(), 0
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk); size += len(chunk)
    return digest.hexdigest(), size

def png_size(path: Path) -> tuple[int, int] | None:
    try:
        with path.open("rb") as f:
            raw = f.read(24)
            if raw[:8] != b"\x89PNG\r\n\x1a\n" or raw[12:16] != b"IHDR": return None
            return int.from_bytes(raw[16:20], "big"), int.from_bytes(raw[20:24], "big")
    except (OSError, ValueError): return None

def extract(source: Path, output: Path, max_files: int = 100_000, max_bytes: int = 20 * 1024**3) -> dict:
    source = source.resolve()
    if not source.is_dir(): raise ValueError("source must be an ExportedProject directory")
    records, counts, total = [], {}, 0
    for root, dirs, files in os.walk(source, followlinks=False):
        dirs[:] = sorted(d for d in dirs if not (Path(root) / d).is_symlink() and d not in {".git", "Library", "Temp"})
        for name in sorted(files):
            path = Path(root) / name
            if path.is_symlink() or path.suffix.lower() not in EXTENSIONS: continue
            if len(records) >= max_files: raise ValueError("file traversal limit exceeded")
            kind = EXTENSIONS[path.suffix.lower()]
            digest, size = sha256(path)
            total += size
            if total > max_bytes: raise ValueError("byte traversal limit exceeded")
            relative = path.relative_to(source).as_posix()
            logical = "source." + relative.replace("/", ".").replace(" ", "_")
            record = {"logical_id": logical, "kind": kind, "source_sha256": digest,
                      "output_sha256": digest, "import_mode": "translate",
                      "importer_version": SCHEMA, "blob": "metadata-only/" + digest,
                      # Relative provenance is safe to publish; absolute source paths are not.
                      "source_path": relative}
            if kind == "image" and path.suffix.lower() == ".png":
                size_info = png_size(path)
                if size_info:
                    record["image"] = {"width": size_info[0], "height": size_info[1],
                                       "color_type": "source", "pixel_sha256": digest, "alpha_bounds": None}
            records.append(record); counts[kind] = counts.get(kind, 0) + 1
    ids = sorted(r["logical_id"] for r in records)
    refs = [{"from": i, "to": i, "kind": "metadata"} for i in ids]
    content = {"schema": "kg_ddlc_plus.content.v1", "roots": ids, "nodes": ids,
               "assets": ids, "locales": [], "stories": [], "references": refs,
               "reachability": {"schema": "kg_ddlc_plus.reachability.v1", "roots": ids,
                 "reachable": ids, "unreachable": [], "dangling": []}, "package_sha256": HEX0}
    encoded = json.dumps({"schema": SCHEMA, "source": str(source), "files": len(records),
                          "bytes": total, "counts": counts}, sort_keys=True).encode()
    content["package_sha256"] = hashlib.sha256(encoded).hexdigest()
    blobs = sorted({"metadata-only/" + r["source_sha256"] for r in records})
    metadata = {"identity": {"id": "kg_ddlc_plus", "display_name": "KG DDLC Plus", "version": "0.1.0"},
      "viewport": {"width": 1920, "height": 1080}, "persistence": {"namespace": "kg_ddlc_plus", "schema": "kg_ddlc_plus.state.v1"},
      "catalog": {"schema": "keygen.assets.v1", "importer_version": SCHEMA, "blobs": blobs, "assets": records},
      "stories": [], "locales": [], "content": content}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    return {"schema": SCHEMA, "files": len(records), "bytes": total, "counts": counts, "output": str(output)}

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", nargs="?", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--max-files", type=int, default=100_000)
    parser.add_argument("--max-bytes", type=int, default=20 * 1024**3)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "ExportedProject"; root.mkdir(); (root / "Assets").mkdir()
            (root / "Assets" / "note.txt").write_text("synthetic\n")
            out = Path(tmp) / "metadata.json"; result = extract(root, out)
            assert result["files"] == 1 and json.loads(out.read_text())["catalog"]["assets"][0]["kind"] == "text"
        print("metadata extractor self-test: ok"); return
    if not args.source or not args.output: parser.error("SOURCE and --output are required")
    print(json.dumps(extract(args.source, args.output, args.max_files, args.max_bytes), sort_keys=True))

if __name__ == "__main__": main()
