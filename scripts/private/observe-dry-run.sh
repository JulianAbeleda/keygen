#!/bin/sh
set -eu

# This command intentionally performs no source discovery and no writes to a
# game installation. It validates the operator's redacted observation file.
record=${1:-}
if [ -z "$record" ] || [ ! -f "$record" ]; then
  echo "usage: $0 REDACTED_RECORD" >&2
  exit 2
fi
case "$record" in
  */docs/evidence/*|*/scripts/private/*) ;;
  *) echo "refusing non-redacted/private record location" >&2; exit 2 ;;
esac
python3 "$(dirname "$0")/../../check-private-content.py" --path "$record"
echo "read-only observation dry-run passed: $record"
