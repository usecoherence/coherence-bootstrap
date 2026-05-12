#!/usr/bin/env bash
# Import reverse-spec PR-2 rows: SPEC coredb-cli-command-surface + five ACs (CLI inventory claims).
# Run from repo root inside isolated test profile (see README in this directory).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT"

if [ -n "${COHERENCE_CORE_DB_BIN:-}" ]; then
  run_cli() { "${COHERENCE_CORE_DB_BIN}" "$@"; }
else
  run_cli() { cargo run -q --manifest-path "$ROOT/Cargo.toml" -p coherence-core-db -- "$@"; }
fi

echo "pr2-import: repo root $ROOT"

run_cli spec add \
  --id CLI-COMMAND-SURFACE-SPEC \
  --slug coredb-cli-command-surface \
  --title "CLI command surface (reverse-spec)" \
  --description "Observed v0 CLI: top-level router, command list, help/version/unknown exit conventions, core vs smoke. Markdown inventory: .coherence/reverse-spec/cli/. Classification: observed." \
  --level system \
  --status draft

run_cli ac add \
  --spec-id CLI-COMMAND-SURFACE-SPEC \
  --id AC-CLI-SURFACE-HELP \
  --slug help-command-exposes-surface \
  --title "Help exposes command surface" \
  --intent "Observed: default argv or help/--help/-h prints full command list and workflow hints to stdout; exits 0." \
  --review-mode manual \
  --risk-level low \
  --concern correctness

run_cli ac add \
  --spec-id CLI-COMMAND-SURFACE-SPEC \
  --id AC-CLI-SURFACE-VERSION \
  --slug version-command-prints-version \
  --title "Version command prints version" \
  --intent "Observed: version/--version/-V prints coherence-core-db version line to stdout; exits 0." \
  --review-mode manual \
  --risk-level low \
  --concern correctness

run_cli ac add \
  --spec-id CLI-COMMAND-SURFACE-SPEC \
  --id AC-CLI-SURFACE-UNKNOWN \
  --slug unknown-command-exits-64 \
  --title "Unknown top-level command exits 64" \
  --intent "Observed: unrecognized argv[1] prints unknown command to stderr, hints help, exits 64 (distinct from typical command failure exit 1)." \
  --review-mode manual \
  --risk-level medium \
  --concern correctness

run_cli ac add \
  --spec-id CLI-COMMAND-SURFACE-SPEC \
  --id AC-CLI-SURFACE-ROUTER \
  --slug top-level-command-groups-are-routed \
  --title "Top-level commands route to command modules" \
  --intent "Observed: known top-level command tokens invoke their corresponding command behavior; subcommands are parsed inside command-specific handlers. Unknown top-level tokens are handled by the unknown-command contract." \
  --review-mode manual \
  --risk-level medium \
  --concern correctness

run_cli ac add \
  --spec-id CLI-COMMAND-SURFACE-SPEC \
  --id AC-CLI-SURFACE-SMOKE \
  --slug smoke-commands-are-internal-candidates \
  --title "Smoke commands are internal/diagnostic candidates" \
  --intent "Observed: m0-smoke and m1-spec-smoke exist for vertical-slice proofs; require isolated test world; classified as internal candidates vs primary operator surface (see cli/commands/smoke-debug.md)." \
  --review-mode manual \
  --risk-level low \
  --concern maintainability

echo "pr2-import: done"
