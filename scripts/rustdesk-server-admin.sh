#!/usr/bin/env bash
# Start rustdesk-server-admin with environment from an optional env file.
# Usage:
#   ./rustdesk-server-admin.sh
#   RUSTDESK_SERVER_ADMIN_ENV=/path/to/config.env ./rustdesk-server-admin.sh
#   RUSTDESK_SERVER_ADMIN_BIN=/opt/rustdesk/rustdesk-server-admin ./rustdesk-server-admin.sh
set -euo pipefail

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_repo_root="$(cd "${_script_dir}/.." && pwd)"

# Optional env file (export KEY=value lines). Override with RUSTDESK_SERVER_ADMIN_ENV.
_env_file="${RUSTDESK_SERVER_ADMIN_ENV:-}"
if [[ -z "${_env_file}" ]]; then
  for _c in "/etc/rustdesk-server-admin.env" "${_script_dir}/rustdesk-server-admin.env"; do
    if [[ -f "${_c}" ]]; then
      _env_file="${_c}"
      break
    fi
  done
fi
if [[ -n "${_env_file}" ]]; then
  if [[ ! -f "${_env_file}" ]]; then
    echo "Env file not found: ${_env_file}" >&2
    exit 1
  fi
  set -a
  # shellcheck disable=SC1090
  source "${_env_file}"
  set +a
fi

# Binary path: explicit env, then common locations.
_bin="${RUSTDESK_SERVER_ADMIN_BIN:-}"
if [[ -z "${_bin}" ]]; then
  for _c in \
    "${_script_dir}/rustdesk-server-admin" \
    "${_repo_root}/target/release/rustdesk-server-admin" \
    "/opt/rustdesk/rustdesk-server-admin" \
    "/usr/local/bin/rustdesk-server-admin"
  do
    if [[ -x "${_c}" ]]; then
      _bin="${_c}"
      break
    fi
  done
fi

if [[ -z "${_bin}" ]] || [[ ! -x "${_bin}" ]]; then
  echo "rustdesk-server-admin: no executable found." >&2
  echo "Install the binary or set RUSTDESK_SERVER_ADMIN_BIN to its full path." >&2
  exit 1
fi

if [[ -z "${ADMIN_PASSWORD:-}" ]]; then
  echo "ADMIN_PASSWORD must be set (in the environment or env file)." >&2
  exit 1
fi

exec "${_bin}" "$@"
