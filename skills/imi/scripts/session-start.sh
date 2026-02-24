#!/usr/bin/env bash
# IMI session bootstrap — runs status + audit in parallel for fast context load.
# Call this at the start of every agent session when imi is detected.
# Takes ~10ms total (same as a single command, both run concurrently).

set -euo pipefail

STATUS_OUT=$(mktemp)
AUDIT_OUT=$(mktemp)
trap 'rm -f "$STATUS_OUT" "$AUDIT_OUT"' EXIT

# Fire both commands in parallel
imi status > "$STATUS_OUT" 2>&1 &
PID_STATUS=$!

imi audit  > "$AUDIT_OUT"  2>&1 &
PID_AUDIT=$!

wait "$PID_STATUS" "$PID_AUDIT"

echo "── IMI STATUS ──────────────────────────────────────────"
cat "$STATUS_OUT"

echo ""
echo "── IMI AUDIT ───────────────────────────────────────────"
cat "$AUDIT_OUT"

echo ""
echo "────────────────────────────────────────────────────────"
echo "Context loaded. Map the user's request to a goal above."
echo "If no matching goal exists, say so before acting."
