#!/bin/bash
set -euo pipefail

APP_CONTENTS="$(cd "$(dirname "$0")/.." && pwd)"
BIN_PATH="$APP_CONTENTS/Resources/bin/oswispa"

if [ ! -x "$BIN_PATH" ]; then
  /usr/bin/osascript -e 'display alert "OSWispa is incomplete" message "The bundled OSWispa binary could not be found." as critical'
  exit 1
fi

# Diagnostics and automated package checks run the bundled binary directly so
# they do not require Terminal automation. Normal Finder launches still open a
# visible Terminal window for first-run setup and status output.
if [ "$#" -gt 0 ]; then
  exec "$BIN_PATH" "$@"
fi

exec /usr/bin/osascript - "$BIN_PATH" <<'APPLESCRIPT'
on run argv
  set binPath to item 1 of argv
  set quotedBinPath to quoted form of binPath

  tell application "Terminal"
    activate
    do script quotedBinPath
  end tell
end run
APPLESCRIPT
