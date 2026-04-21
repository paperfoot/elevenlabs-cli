#!/usr/bin/env bash
# Refresh the ElevenLabs OpenAPI snapshot + derived inventories.
# Run from the repo root or anywhere — resolves paths relative to this
# script's own directory.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
SPEC="$HERE/openapi.elevenlabs.json"
ENDPOINTS="$HERE/endpoints-inventory.txt"
CLI_ENDPOINTS="$HERE/cli-endpoints.txt"

UPSTREAM="${ELEVENLABS_OPENAPI_URL:-https://api.elevenlabs.io/openapi.json}"

echo "→ fetching $UPSTREAM"
curl -fsSL "$UPSTREAM" -o "$SPEC"
bytes=$(wc -c < "$SPEC" | tr -d ' ')
echo "  saved $SPEC ($bytes bytes)"

echo "→ regenerating $ENDPOINTS"
python3 - "$SPEC" "$ENDPOINTS" <<'PY'
import json, sys
spec_path, out_path = sys.argv[1], sys.argv[2]
with open(spec_path) as f:
    spec = json.load(f)
lines = []
for path, methods in sorted(spec.get("paths", {}).items()):
    for method, op in methods.items():
        if method.startswith("x-"):
            continue
        summary = (op.get("summary") or "").strip().replace("\n", " ")
        op_id = op.get("operationId", "")
        tag = (op.get("tags") or [""])[0]
        lines.append(f"{method.upper():7} {path:90} [{tag}] {op_id}  — {summary}")
with open(out_path, "w") as f:
    f.write("\n".join(lines) + "\n")
print(f"  {len(lines)} operations")
PY

echo "→ regenerating $CLI_ENDPOINTS"
python3 - "$REPO" "$CLI_ENDPOINTS" <<'PY'
import os, re, sys
repo, out_path = sys.argv[1], sys.argv[2]
endpoints = set()
for root, _, files in os.walk(os.path.join(repo, "src")):
    for f in files:
        if not f.endswith(".rs"):
            continue
        full = os.path.join(root, f)
        rel = os.path.relpath(full, repo)
        with open(full) as fh:
            text = fh.read()
        for m in re.finditer(r'"(/v1/[^"\s\\]+)"', text):
            raw = m.group(1)
            norm = re.sub(r"\{[^}]+\}", "{X}", raw)
            endpoints.add((norm, rel))
with open(out_path, "w") as f:
    for e, src in sorted(endpoints):
        f.write(f"{e:80s}  -- {src}\n")
    f.write(f"\nTotal unique: {len({e for e,_ in endpoints})}\n")
print(f"  {len({e for e,_ in endpoints})} unique endpoints")
PY

echo "done."
