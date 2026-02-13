#!/bin/bash
set -euo pipefail

COMMAND="${1:-toggle}"

if [[ -n "${XDG_RUNTIME_DIR:-}" ]]; then
  SOCKET_PATH="${XDG_RUNTIME_DIR}/oswispa.sock"
else
  SOCKET_PATH="/tmp/oswispa-$(id -u).sock"
fi

if [[ ! -S "${SOCKET_PATH}" ]]; then
  echo "OSWispa socket not found at ${SOCKET_PATH}. Is OSWispa running?" >&2
  exit 1
fi

if command -v nc >/dev/null 2>&1; then
  printf "%s" "${COMMAND}" | nc -U "${SOCKET_PATH}"
  exit 0
fi

if command -v socat >/dev/null 2>&1; then
  printf "%s" "${COMMAND}" | socat - "UNIX-CONNECT:${SOCKET_PATH}"
  exit 0
fi

echo "Neither nc nor socat is installed; cannot send OSWispa toggle command." >&2
exit 1
