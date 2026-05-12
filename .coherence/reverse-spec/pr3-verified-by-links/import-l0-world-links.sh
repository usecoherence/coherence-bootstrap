#!/usr/bin/env bash
# PR-3.5: upsert codeintel rows so verify-ac runs world/bin/run-ac for three L0 ACs.
# Requires PR-2 import in the same Dolt logical catalog. No crates/** changes.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

if [ -n "${COHERENCE_CORE_DB_BIN:-}" ]; then
  run_cli() { "${COHERENCE_CORE_DB_BIN}" "$@"; }
else
  run_cli() { cargo run -q --manifest-path "$ROOT/Cargo.toml" -p coherence-core-db -- "$@"; }
fi

resolve_catalog_name() {
  if [ -n "${DOLT_DB:-}" ]; then
    printf '%s\n' "$DOLT_DB"
    return
  fi
  # Avoid `doctor | awk`: Rust stdout panics on SIGPIPE when the reader exits early (Broken pipe).
  local doc_out
  doc_out="$(mktemp)"
  run_cli doctor >"$doc_out" 2>&1 || true
  awk -F': ' '/^effective_catalog_without_DOLT_DB_override:/ {print $2; exit}' "$doc_out"
  rm -f "$doc_out"
}

reverse_spec_run_sql_batch() {
  local sql_file="$1"
  if [ -n "${COHERENCE_REVERSE_SPEC_SQL:-}" ]; then
    "${COHERENCE_REVERSE_SPEC_SQL}" <"$sql_file"
    return
  fi
  if command -v mysql >/dev/null 2>&1 && [ -n "${DOLT_SOCKET:-}" ]; then
    mysql --protocol=SOCKET -S "$DOLT_SOCKET" -u "${DOLT_USER:-root}" ${DOLT_PASSWORD:+-p"$DOLT_PASSWORD"} \
      --database="$DB_NAME" <"$sql_file"
    return
  fi
  if command -v mariadb >/dev/null 2>&1 && [ -n "${DOLT_SOCKET:-}" ]; then
    mariadb --protocol=SOCKET -S "$DOLT_SOCKET" -u "${DOLT_USER:-root}" ${DOLT_PASSWORD:+-p"$DOLT_PASSWORD"} \
      --database="$DB_NAME" <"$sql_file"
    return
  fi

  local host port
  host="${DOLT_HOST:-127.0.0.1}"
  port="${DOLT_PORT:-}"
  if [ -z "$port" ] && [ -r "$ROOT/.coherence/run/dolt.tcp_port" ]; then
    port="$(tr -d '\n\r ' <"$ROOT/.coherence/run/dolt.tcp_port")"
  fi
  port="${port:-3306}"

  dolt --no-tls --host "$host" --port "$port" --user "${DOLT_USER:-root}" -p "${DOLT_PASSWORD:-}" sql <"$sql_file"
}

DB_NAME="$(resolve_catalog_name)"
if [ -z "$DB_NAME" ]; then
  echo "pr3-import: could not resolve catalog (set DOLT_DB or ensure doctor prints effective_catalog_without_DOLT_DB_override)" >&2
  exit 1
fi

echo "pr3-import: repo root $ROOT"
echo "pr3-import: logical catalog (database) $DB_NAME"

TS="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

# shellcheck disable=SC2016
# repo_path '' => verify-ac inherits operator cwd (repo root); see ac_verify.rs resolve_working_dir.
cat >"$tmp" <<EOF
USE \`${DB_NAME}\`;

INSERT INTO codeintel_code_locations (
  id, repo_path, file_path, kind, symbol, test_command, created_at, updated_at
) VALUES
  (
    'cl-rs-l0-help',
    '',
    '.coherence/reverse-spec/world/bin/run-ac',
    'test_command',
    NULL,
    '.coherence/reverse-spec/world/bin/run-ac AC-CLI-SURFACE-HELP',
    '${TS}',
    '${TS}'
  ),
  (
    'cl-rs-l0-version',
    '',
    '.coherence/reverse-spec/world/bin/run-ac',
    'test_command',
    NULL,
    '.coherence/reverse-spec/world/bin/run-ac AC-CLI-SURFACE-VERSION',
    '${TS}',
    '${TS}'
  ),
  (
    'cl-rs-l0-unknown',
    '',
    '.coherence/reverse-spec/world/bin/run-ac',
    'test_command',
    NULL,
    '.coherence/reverse-spec/world/bin/run-ac AC-CLI-SURFACE-UNKNOWN',
    '${TS}',
    '${TS}'
  )
ON DUPLICATE KEY UPDATE
  repo_path = VALUES(repo_path),
  file_path = VALUES(file_path),
  kind = VALUES(kind),
  symbol = VALUES(symbol),
  test_command = VALUES(test_command),
  updated_at = VALUES(updated_at);

INSERT INTO codeintel_ac_links (
  id, ac_id, code_location_id, relation_kind, note, created_at, updated_at
) VALUES
  ('acl-rs-l0-help', 'AC-CLI-SURFACE-HELP', 'cl-rs-l0-help', 'verified_by', 'PR-3.5 reverse-spec: L0 help to world harness', '${TS}', '${TS}'),
  ('acl-rs-l0-version', 'AC-CLI-SURFACE-VERSION', 'cl-rs-l0-version', 'verified_by', 'PR-3.5 reverse-spec: L0 version to world harness', '${TS}', '${TS}'),
  ('acl-rs-l0-unknown', 'AC-CLI-SURFACE-UNKNOWN', 'cl-rs-l0-unknown', 'verified_by', 'PR-3.5 reverse-spec: L0 unknown to world harness', '${TS}', '${TS}')
ON DUPLICATE KEY UPDATE
  ac_id = VALUES(ac_id),
  code_location_id = VALUES(code_location_id),
  relation_kind = VALUES(relation_kind),
  note = VALUES(note),
  updated_at = VALUES(updated_at);
EOF

reverse_spec_run_sql_batch "$tmp"
echo "pr3-import: done"
