#!/usr/bin/env bash
# IMI session-start hook — inject state context before agent responds
# Gracefully skips if .imi/ not initialized or imi not in PATH

IMI_BIN="$(command -v imi 2>/dev/null)"

if [[ -z "$IMI_BIN" ]]; then
  exit 0
fi

DB_PATH=".imi/state.db"
if [[ ! -f "$DB_PATH" ]]; then
  echo "IMI not initialized in this project. Run: imi init"
  exit 0
fi

"$IMI_BIN" status 2>/dev/null
echo "---"
"$IMI_BIN" context 2>/dev/null
