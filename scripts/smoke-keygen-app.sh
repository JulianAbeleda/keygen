#!/bin/sh
set -eu
app="${1:?usage: $0 APP_BUNDLE}"
target="${2:?usage: $0 APP_BUNDLE TARGET}"
bundle_id="${3:?usage: $0 APP_BUNDLE TARGET BUNDLE_ID}"
[ "$(uname -s)" = Darwin ] || { echo "macOS required" >&2; exit 1; }
[ -d "$app" ] || { echo "bundle missing: $app" >&2; exit 1; }
bin="$app/Contents/MacOS/$target"
plist="$app/Contents/Info.plist"
manifest="$app/Contents/Resources/package-manifest.json"
[ -x "$bin" ] && [ -f "$plist" ] && [ -f "$manifest" ]
file -b "$bin" | grep -q arm64
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist" | grep -qx "$bundle_id"
python3 - "$manifest" <<'PY'
import hashlib, json, pathlib, sys
p = pathlib.Path(sys.argv[1]); d = json.loads(p.read_text())
assert d['schema'] == 'keygen.macos.package.v1' and d['arch'] == 'arm64'
root = p.parents[2]
for item in d['files']:
    q = root / item['path']
    assert q.is_file() and hashlib.sha256(q.read_bytes()).hexdigest() == item['sha256'], q
print('generic macOS app smoke OK')
PY
