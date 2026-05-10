#!/usr/bin/env bash
# Shared paths for ADR-0006 user-scoped Dolt (coherence-core-db).
# shellcheck disable=SC2034  # library; callers use functions

coherence_use_user_scoped_dolt() {
  case "${COHERENCE_USE_USER_SCOPED_DOLT:-}" in
  1 | true | TRUE | yes | YES) return 0 ;;
  *) return 1 ;;
  esac
}

coherence_xdg_data_home() {
  echo "${XDG_DATA_HOME:-$HOME/.local/share}"
}

# ADR-0006: single sql-server data-dir holding every project database.
coherence_dolt_data_dir() {
  echo "${COHERENCE_DOLT_DATA_DIR:-$(coherence_xdg_data_home)/coherence/db}"
}

# Runtime directory for socket, pid, log, internal TCP port metadata.
coherence_dolt_runtime_dir() {
  if [ -n "${COHERENCE_DOLT_RUNTIME_DIR:-}" ]; then
    echo "${COHERENCE_DOLT_RUNTIME_DIR}"
    return
  fi
  if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    echo "${XDG_RUNTIME_DIR}/coherence"
  else
    echo "${HOME}/.cache/coherence/run"
  fi
}

# Listener socket for user-scoped service (override with DOLT_SOCKET).
coherence_user_scoped_socket_path() {
  local rt
  rt="$(coherence_dolt_runtime_dir)"
  echo "${DOLT_SOCKET:-$rt/dolt.sock}"
}

# Internal TCP listener for Refinery/migrate (mysql crate URL); not primary operator UX.
coherence_user_scoped_tcp_port() {
  echo "${COHERENCE_DOLT_TCP_PORT:-33306}"
}
