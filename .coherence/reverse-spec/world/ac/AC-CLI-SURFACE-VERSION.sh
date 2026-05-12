#!/usr/bin/env bash
# L0: version line (no Dolt).
set -euo pipefail
: "${COHERENCE_CORE_DB_BIN:?}" "${EVIDENCE_DIR:?}" "${ROOT:?}"

"$COHERENCE_CORE_DB_BIN" version >"$EVIDENCE_DIR/stdout.txt" 2>"$EVIDENCE_DIR/stderr.txt"
code=$?
printf '%s' "$code" >"$EVIDENCE_DIR/exit.code"
if [[ "$code" -ne 0 ]]; then
  echo "AC-CLI-SURFACE-VERSION: expected exit 0, got $code" >&2
  exit 1
fi
if ! grep -q 'coherence-core-db' "$EVIDENCE_DIR/stdout.txt"; then
  echo "AC-CLI-SURFACE-VERSION: stdout missing coherence-core-db" >&2
  exit 1
fi
