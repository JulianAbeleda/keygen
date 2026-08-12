#!/bin/sh
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
out="$repo_root/dist/macos"
binary=""
resources=""
usage() { echo "usage: $0 --binary PATH [--resources DIR] [--out DIR]" >&2; exit 2; }
while [ "$#" -gt 0 ]; do
  case "$1" in
    --binary) [ "$#" -ge 2 ] || usage; binary=$2; shift 2 ;;
    --resources) [ "$#" -ge 2 ] || usage; resources=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    -h|--help) usage ;;
    *) echo "unknown option: $1" >&2; usage ;;
  esac
done
[ -n "$binary" ] || usage
[ "$(uname -s)" = Darwin ] || { echo "kg_ddlc_plus packaging requires macOS" >&2; exit 1; }
[ "$(uname -m)" = arm64 ] || { echo "kg_ddlc_plus packaging requires an arm64 host" >&2; exit 1; }
[ -f "$binary" ] || { echo "binary not found: $binary" >&2; exit 1; }
binary=$(CDPATH= cd -- "$(dirname -- "$binary")" && pwd)/$(basename -- "$binary")
case "$(file -b "$binary")" in *arm64*) ;; *) echo "binary is not arm64: $binary" >&2; exit 1 ;; esac
app="$out/kg_ddlc_plus.app"
contents="$app/Contents"
rm -rf "$app"
mkdir -p "$contents/MacOS" "$contents/Resources"
cp "$binary" "$contents/MacOS/kg_ddlc_plus"
chmod 755 "$contents/MacOS/kg_ddlc_plus"
if [ -n "$resources" ]; then
  [ -d "$resources" ] || { echo "resources directory not found: $resources" >&2; exit 1; }
  cp -R "$resources" "$contents/Resources/package"
fi
cat > "$contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDisplayName</key><string>kg_ddlc_plus</string>
<key>CFBundleExecutable</key><string>kg_ddlc_plus</string>
<key>CFBundleIdentifier</key><string>com.julian.keygen.kg-ddlc-plus</string>
<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
<key>CFBundleName</key><string>kg_ddlc_plus</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>0.1.0</string>
<key>LSMinimumSystemVersion</key><string>15.0</string>
<key>LSArchitecturePriority</key><array><string>arm64</string></array>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
python3 - "$app" <<'PY'
import hashlib, json, pathlib, sys
root = pathlib.Path(sys.argv[1])
files = [{'path': p.relative_to(root).as_posix(), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
         for p in sorted(root.rglob('*')) if p.is_file() and p.name != 'package-manifest.json']
(root / 'Contents' / 'Resources' / 'package-manifest.json').write_text(
    json.dumps({'schema': 'keygen.macos.package.v1', 'bundle_id': 'com.julian.keygen.kg-ddlc-plus',
                'target': 'kg_ddlc_plus', 'arch': 'arm64', 'files': files}, indent=2) + '\n', encoding='utf-8')
PY
echo "packaged $app"
