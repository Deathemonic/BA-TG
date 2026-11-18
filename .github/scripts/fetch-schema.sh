#!/usr/bin/env bash
set -euo pipefail

: "${FBS_REPO:?Missing FBS_REPO}"
: "${FBS_TOKEN:?Missing FBS_REPO_PAT}"
: "${FBS_NS_ROOT:?Missing FBS_NS_ROOT}"
: "${SCHEMA_SHA:?Missing pinned schema SHA}"
: "${REGION:?Missing schema region}"

if [[ ! "$SCHEMA_SHA" =~ ^[0-9a-fA-F]{40}$ ]]; then
  echo "Invalid schema commit SHA." >&2
  exit 1
fi

case "$REGION" in
  japan) schema_file=Japan.fbs; region_namespace=Japan ;;
  global) schema_file=Global.fbs; region_namespace=Global ;;
  *) echo "Unsupported schema region." >&2; exit 1 ;;
esac

schema_dir="${1:-schema}"
mkdir -p "$schema_dir"
raw_schema=$(mktemp "$schema_dir/.raw-XXXXXX")
trap 'rm -f "$raw_schema"' EXIT

curl -fsSL -H "Authorization: Bearer $FBS_TOKEN" \
  "https://raw.githubusercontent.com/$FBS_REPO/$SCHEMA_SHA/$schema_file" \
  -o "$raw_schema"

python - "$FBS_NS_ROOT" "$region_namespace" "$raw_schema" "$schema_dir/table.fbs" <<'PY'
import re
import sys
from pathlib import Path

ns_root, region, in_path, out_path = sys.argv[1:]
prefix = f"{ns_root}.{region}"
pattern = re.compile(r"^\s*namespace\s+" + re.escape(prefix) + r"(\.[A-Za-z0-9_.]+)?\s*;\s*$")
text = Path(in_path).read_text(encoding="utf-8-sig")
out_lines = []
matched = False

for line in text.splitlines():
    match = pattern.match(line)
    if match:
        matched = True
        rest = match.group(1)
        if rest:
            out_lines.append(f"namespace {rest.lstrip('.')};")
    else:
        out_lines.append(line)

if not matched:
    sys.exit("Schema namespace does not match FBS_NS_ROOT and region.")

result = re.sub(r"\n{3,}", "\n\n", "\n".join(out_lines)).strip() + "\n"
for namespace in ("FlatData", "MX.Data.Excel"):
    if not re.search(r"^\s*namespace\s+" + re.escape(namespace) + r"\s*;", result, re.MULTILINE):
        sys.exit("Schema is missing a required table or row namespace.")

Path(out_path).write_text(result, encoding="utf-8", newline="\n")
PY
