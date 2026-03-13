#!/bin/bash
set -euo pipefail

APP_CONTENTS="$(cd "$(dirname "$0")/.." && pwd)"
BIN_PATH="$APP_CONTENTS/Resources/bin/oswispa"

if [ ! -x "$BIN_PATH" ]; then
  /usr/bin/osascript -e 'display alert "OSWispa is incomplete" message "The bundled OSWispa binary could not be found." as critical'
  exit 1
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
