#!/usr/bin/env bash
set -euo pipefail

: "${DEPLOY_HOST:?Set DEPLOY_HOST (e.g. 203.0.113.10)}"
: "${DEPLOY_USER:?Set DEPLOY_USER (e.g. deploy)}"
: "${DEPLOY_PATH:?Set DEPLOY_PATH (e.g. /var/www/oswispa.tylerbuilds.com)}"

ssh_cmd=(ssh)
if [[ -n "${DEPLOY_PORT:-}" ]]; then
  ssh_cmd+=(-p "$DEPLOY_PORT")
fi
if [[ -n "${DEPLOY_KEY:-}" ]]; then
  ssh_cmd+=(-i "$DEPLOY_KEY")
fi

rsync -rlz --delete \
  --no-perms --no-owner --no-group \
  -e "${ssh_cmd[*]}" \
  website/ "${DEPLOY_USER}@${DEPLOY_HOST}:${DEPLOY_PATH}/"

"${ssh_cmd[@]}" "${DEPLOY_USER}@${DEPLOY_HOST}" \
  "find '${DEPLOY_PATH}' -type d -exec chmod 755 {} + && find '${DEPLOY_PATH}' -type f -exec chmod 644 {} +"
