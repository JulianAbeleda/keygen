#!/bin/sh
set -eu
# Generic KeyGen arm64 bundle boundary; target identity is supplied by caller.
out=dist/macos; binary=; target=; display_name=; bundle_id=; version=0.1.0; min_os=15.0; resources=
usage() { echo "usage: $0 --binary PATH --target ID --display-name NAME --bundle-id ID [--resources DIR] [--out DIR]" >&2; exit 2; }
while [ "$#" -gt 0 ]; do
  case "$1" in
    --binary) [ "$#" -ge 2 ] || usage; binary=$2; shift 2 ;;
    --target) [ "$#" -ge 2 ] || usage; target=$2; shift 2 ;;
    --display-name) [ "$#" -ge 2 ] || usage; display_name=$2; shift 2 ;;
    --bundle-id) [ "$#" -ge 2 ] || usage; bundle_id=$2; shift 2 ;;
    --resources) [ "$#" -ge 2 ] || usage; resources=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    --version) [ "$#" -ge 2 ] || usage; version=$2; shift 2 ;;
    --min-os) [ "$#" -ge 2 ] || usage; min_os=$2; shift 2 ;;
    -h|--help) usage ;;
    *) echo "unknown option: $1" >&2; usage ;;
  esac
done
[ -n "$binary" ] && [ -n "$target" ] && [ -n "$display_name" ] && [ -n "$bundle_id" ] || usage
[ "$(uname -s)" = Darwin ] || { echo "KeyGen packaging requires macOS" >&2; exit 1; }
[ "$(uname -m)" = arm64 ] || { echo "KeyGen packaging requires arm64" >&2; exit 1; }
[ -f "$binary" ] || { echo "binary not found: $binary" >&2; exit 1; }
case "$(file -b "$binary")" in *arm64*) ;; *) echo "binary is not arm64" >&2; exit 1 ;; esac
case "$target" in *[!A-Za-z0-9_-]*|'') echo "invalid target" >&2; exit 1 ;; esac
case "$bundle_id" in *[!A-Za-z0-9.-]*|'') echo "invalid bundle id" >&2; exit 1 ;; esac
app="$out/$target.app"; contents="$app/Contents"
rm -rf "$app"; mkdir -p "$contents/MacOS" "$contents/Resources"
cp "$binary" "$contents/MacOS/$target"; chmod 755 "$contents/MacOS/$target"
[ -z "$resources" ] || { [ -d "$resources" ] || exit 1; cp -R "$resources" "$contents/Resources/package"; }
# Finder launches provide no scene argument.  Every packaged project therefore
# gets a deterministic boot alias while retaining its original scene files.
# A project may provide an explicit boot.json; otherwise the first scene is
# copied beside it so relative asset paths remain valid.
if [ -d "$contents/Resources/package/scenes" ] && [ ! -f "$contents/Resources/package/scenes/boot.json" ]; then
  first_scene=$(find "$contents/Resources/package/scenes" -maxdepth 1 -type f -name '*.json' ! -name 'boot.json' -print | sort | head -n 1)
  [ -z "$first_scene" ] || cp "$first_scene" "$contents/Resources/package/scenes/boot.json"
fi
python3 - "$app" "$target" "$display_name" "$bundle_id" "$version" "$min_os" <<'PY'
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1]); c=root/'Contents'; target,name,bundle,version,min_os=sys.argv[2:]
def tag(k,v): return '<key>%s</key><string>%s</string>\n' % (k,v)
vals={'CFBundleDisplayName':name,'CFBundleExecutable':target,'CFBundleIdentifier':bundle,'CFBundleInfoDictionaryVersion':'6.0','CFBundleName':name,'CFBundleShortVersionString':version,'CFBundleVersion':version}
plist='<?xml version="1.0" encoding="UTF-8"?>\n<plist version="1.0"><dict>\n'+''.join(tag(k,v) for k,v in vals.items())+'<key>CFBundlePackageType</key><string>APPL</string>\n<key>LSMinimumSystemVersion</key><string>'+min_os+'</string>\n<key>LSMinimumSystemVersionByArchitecture</key><dict><key>arm64</key><string>'+min_os+'</string></dict>\n<key>LSArchitecturePriority</key><array><string>arm64</string></array>\n<key>NSHighResolutionCapable</key><true/>\n<key>NSPrincipalClass</key><string>NSApplication</string>\n</dict></plist>\n'
(c/'Info.plist').write_text(plist)
files=[{'path':p.relative_to(root).as_posix(),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(root.rglob('*')) if p.is_file() and p.name!='package-manifest.json']
(c/'Resources/package-manifest.json').write_text(json.dumps({'schema':'keygen.macos.package.v1','target':target,'bundle_id':bundle,'arch':'arm64','files':files},indent=2)+'\n')
PY
echo "packaged $app"
