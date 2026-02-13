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

rsync -az --delete \
  --no-owner --no-group \
  --chmod=Du=rwx,Dgo=rx,Fu=rw,Fgo=r \
  -e "${ssh_cmd[*]}" \
  website/ "${DEPLOY_USER}@${DEPLOY_HOST}:${DEPLOY_PATH}/"
