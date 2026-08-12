#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
HERE=$ROOT/examples/sample_project
OUT=$HERE/out
mkdir -p "$HERE/assets" "$OUT"
python3 - "$HERE/assets/sample.png" <<'PY'
import struct, sys, zlib
path = sys.argv[1]
w, h = 960, 540
rows = []
for y in range(h):
    row = bytearray([0])
    for x in range(w):
        row += bytes((18 + x * 20 // w, 24 + y * 30 // h, 36 + (x+y) * 18 // (w+h), 255))
    rows.append(row)
def chunk(name, data):
    return struct.pack('>I', len(data)) + name + data + struct.pack('>I', zlib.crc32(name + data) & 0xffffffff)
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w,h,8,6,0,0,0)) + chunk(b'IDAT', zlib.compress(b''.join(rows), 9)) + chunk(b'IEND', b'')
open(path, 'wb').write(png)
PY
FONT=/System/Library/Fonts/Supplemental/Arial.ttf
[ -f "$FONT" ] || FONT=/System/Library/Fonts/Helvetica.ttc
cp "$FONT" "$HERE/assets/sample.ttf"
python3 - "$HERE/project.json" <<'PY'
import json, sys
p=json.load(open(sys.argv[1]))
assert p['schema']=='keygen.project.v1' and p['project']['id']=='sample.project'
assert p['scenes'][0]['asset_ids']==['image.sample']
print('inspect OK: keygen.project.v1 sample.project')
PY
cargo run -q -p keygen -- --scene "$HERE/scenes/start.json" --validate
rm -f "$OUT/sample.png"
cargo run -q -p keygen -- --scene "$HERE/scenes/start.json" --render "$OUT/sample.png" --time 0
test -s "$OUT/sample.png"
echo "render OK: $OUT/sample.png"
