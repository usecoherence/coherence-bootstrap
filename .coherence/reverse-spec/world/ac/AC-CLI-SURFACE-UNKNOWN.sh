#!/usr/bin/env bash
# L0: unknown top-level command exits 64 (no Dolt).
set -euo pipefail
: "${COHERENCE_CORE_DB_BIN:?}" "${EVIDENCE_DIR:?}" "${ROOT:?}"

set +e
"$COHERENCE_CORE_DB_BIN" __reverse_spec_unknown_command_xyz__ \
  >"$EVIDENCE_DIR/stdout.txt" 2>"$EVIDENCE_DIR/stderr.txt"
code=$?
set -e
printf '%s\n' "$code" >"$EVIDENCE_DIR/exit_code.txt"
if [[ "$code" -ne 64 ]]; then
  echo "AC-CLI-SURFACE-UNKNOWN: expected exit 64, got $code" >&2
  exit 1
fi
if ! grep -q 'unknown command' "$EVIDENCE_DIR/stderr.txt"; then
  echo "AC-CLI-SURFACE-UNKNOWN: stderr missing unknown command" >&2
  exit 1
fi
printf '%s\n' "PASS: exit 64 and stderr mentions unknown command" >"$EVIDENCE_DIR/assertion.txt"
