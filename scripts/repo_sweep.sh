#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

B1="rb""_vm"
B2="ubl_ai""_nrf1"
B3="rb""vm"
B4="docs/vi""sao/"
B5="docs/canon/UNC""-1.md"
B6="docs/life""cycle/"
B7="docs/ops/ROLLOUT""_AUTOMATION.md"
B8="docs/arch""ive/"
B9="artifacts/read""iness/"
B10="/Us""ers/"
B11="LogLine-Foundation/UBL-""CORE"
BANNED_PATTERN="${B1}|${B2}|${B3}|${B4}|${B5}|${B6}|${B7}|${B8}|${B9}|${B10}|${B11}"

echo "[repo-sweep] checking banned legacy patterns"
if rg -n "$BANNED_PATTERN" . \
  --glob '!target/**' \
  --glob '!.git/**'; then
  echo "[repo-sweep] banned pattern(s) found" >&2
  exit 1
fi

echo "[repo-sweep] validating markdown links"
python3 - <<'PY'
from pathlib import Path
import re

root = Path('.').resolve()
md_files = [p for p in root.rglob('*.md') if '.git' not in p.parts]
link_re = re.compile(r'\[[^\]]*\]\(([^)]+)\)')
path_re = re.compile(r'(docs/[A-Za-z0-9_./-]+\.md)')
errors = []

for md in md_files:
    text = md.read_text(encoding='utf-8')
    for m in link_re.finditer(text):
        link = m.group(1).strip()
        if link.startswith(('http://','https://','mailto:','#')):
            continue
        link = link.split('#',1)[0]
        if not link:
            continue
        candidate = (md.parent / link).resolve()
        if not candidate.exists():
            errors.append(f"{md.relative_to(root)} -> missing link target: {link}")
    for m in path_re.finditer(text):
        rel = m.group(1)
        candidate = (root / rel).resolve()
        if not candidate.exists():
            errors.append(f"{md.relative_to(root)} -> missing referenced doc path: {rel}")

if errors:
    for e in errors:
        print(e)
    raise SystemExit(1)
print('ok')
PY

echo "[repo-sweep] ok"
