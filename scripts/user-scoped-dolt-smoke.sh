#!/usr/bin/env bash
# Manual verification flow for ADR-0006 (user-scoped Dolt). Uses isolated dirs under /tmp.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BASE="${COHERENCE_USER_SCOPED_SMOKE_ROOT:-/tmp/coherence-user-scoped-smoke}"
DATA="$BASE/data"
RUNTIME="$BASE/run"

rm -rf "$BASE"
mkdir -p "$DATA" "$RUNTIME"

export COHERENCE_DOLT_DATA_DIR="$DATA"
export COHERENCE_DOLT_RUNTIME_DIR="$RUNTIME"

echo "== dolt-start (user-scoped, isolated) =="
./scripts/dolt-start

export DOLT_SOCKET="${DOLT_SOCKET:-$RUNTIME/dolt.sock}"

echo "== migrate + spec list: proj_one =="
export DOLT_DB=proj_one
./scripts/migrate
cargo run -q -p coherence-core-db -- spec list

echo "== migrate + spec list: proj_two =="
export DOLT_DB=proj_two
./scripts/migrate
cargo run -q -p coherence-core-db -- spec list

echo "== dolt-stop =="
./scripts/dolt-stop

echo "user-scoped-dolt-smoke: ok"
