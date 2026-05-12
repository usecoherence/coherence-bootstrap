#!/usr/bin/env bash
# L0: help exposes surface (no Dolt).
set -euo pipefail
: "${COHERENCE_CORE_DB_BIN:?}" "${EVIDENCE_DIR:?}" "${ROOT:?}"

"$COHERENCE_CORE_DB_BIN" help >"$EVIDENCE_DIR/stdout.txt" 2>"$EVIDENCE_DIR/stderr.txt"
code=$?
printf '%s' "$code" >"$EVIDENCE_DIR/exit.code"
if [[ "$code" -ne 0 ]]; then
  echo "AC-CLI-SURFACE-HELP: expected exit 0, got $code" >&2
  exit 1
fi
if ! grep -q 'verify-ac' "$EVIDENCE_DIR/stdout.txt"; then
  echo "AC-CLI-SURFACE-HELP: stdout missing verify-ac" >&2
  exit 1
fi
