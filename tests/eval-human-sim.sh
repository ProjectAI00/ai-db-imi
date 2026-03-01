#!/usr/bin/env bash
set -u -o pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMI_BIN="${IMI_BIN:-$REPO_ROOT/target/release/imi}"
if [[ ! -x "$IMI_BIN" ]]; then
  IMI_BIN="$(command -v imi 2>/dev/null || true)"
fi

PASS=0
FAIL=0
TMP_DIRS=()
CMD_OUT=""
CMD_EXIT=0
CONTEXT_OUT=""
STATUS_OUT=""

cleanup() {
  for d in "${TMP_DIRS[@]:-}"; do
    [[ -n "$d" && -d "$d" ]] && rm -rf "$d"
  done
}
trap cleanup EXIT

mktemp_dir() {
  local d
  d=$(mktemp -d "/tmp/imi-human-sim-XXXXXX")
  TMP_DIRS+=("$d")
  echo "$d"
}

run_in_dir() {
  local dir="$1"
  shift
  CMD_OUT="$(cd "$dir" && "$@" 2>&1)"
  CMD_EXIT=$?
}

capture_id() {
  echo "$1" | grep -oE '[a-z0-9]{14,}' | head -1
}

check() {
  local label="$1"
  local expected="$2"
  local got="$3"
  if [[ -z "$got" ]]; then
    FAIL=$((FAIL + 1))
    printf "FAIL  %s\n" "$label"
    printf "      expected: %s\n" "$expected"
    printf "      got: (empty)\n"
    return
  fi
  PASS=$((PASS + 1))
  printf "PASS  %s\n" "$label"
}

check_contains() {
  local label="$1"
  local expected="$2"
  local haystack="$3"
  local needle="$4"
  if [[ "$haystack" =~ $needle ]]; then
    PASS=$((PASS + 1))
    printf "PASS  %s\n" "$label"
  else
    FAIL=$((FAIL + 1))
    printf "FAIL  %s\n" "$label"
    printf "      expected: %s\n" "$expected"
    printf "      got:\n%s\n" "$haystack"
  fi
}

check_not_contains() {
  local label="$1"
  local expected="$2"
  local haystack="$3"
  local needle="$4"
  if [[ "$haystack" =~ $needle ]]; then
    FAIL=$((FAIL + 1))
    printf "FAIL  %s\n" "$label"
    printf "      expected: %s\n" "$expected"
    printf "      got:\n%s\n" "$haystack"
  else
    PASS=$((PASS + 1))
    printf "PASS  %s\n" "$label"
  fi
}

echo "=== IMI human-simulation eval ==="
echo "Binary: $IMI_BIN"

if [[ -z "${IMI_BIN:-}" || ! -x "$IMI_BIN" ]]; then
  echo "FAIL  setup"
  echo "      expected: executable IMI binary"
  echo "      got: missing target/release/imi and no imi in PATH"
  exit 1
fi

TMP_PROJECT="$(mktemp_dir)"

run_in_dir "$TMP_PROJECT" "$IMI_BIN" init
check_contains "setup init" "imi init exits 0" "exit=$CMD_EXIT output=$CMD_OUT" "exit=0"
if [[ "$CMD_EXIT" -ne 0 ]]; then
  echo "Eval aborted: init failed"
  exit 1
fi

run_in_dir "$TMP_PROJECT" "$IMI_BIN" add-goal "build a payment API" "ship reliable payment processing"
GOAL_ID="$(capture_id "$CMD_OUT")"
check_contains "setup goal" "add-goal returns goal id" "$CMD_OUT" "[a-z0-9]{14,}"
if [[ "$CMD_EXIT" -ne 0 || -z "$GOAL_ID" ]]; then
  echo "Eval aborted: goal creation failed"
  exit 1
fi

run_in_dir "$TMP_PROJECT" "$IMI_BIN" add-task "$GOAL_ID" "add stripe webhook endpoint" "verify signatures and process event types"
check_contains "setup task 1" "add-task exits 0" "exit=$CMD_EXIT output=$CMD_OUT" "exit=0"

run_in_dir "$TMP_PROJECT" "$IMI_BIN" add-task "$GOAL_ID" "add idempotency key to payments" "prevent duplicate charges on retries"
check_contains "setup task 2" "add-task exits 0" "exit=$CMD_EXIT output=$CMD_OUT" "exit=0"

run_in_dir "$TMP_PROJECT" "$IMI_BIN" context
CONTEXT_OUT="$CMD_OUT"
check "capture context" "non-empty context output from imi context" "$CONTEXT_OUT"

echo ""
echo "--- Captured context (agent-visible) ---"
echo "$CONTEXT_OUT"
echo "----------------------------------------"
echo ""

check_contains \
  "prompt: what should I work on today?" \
  "context includes at least one todo task title" \
  "$CONTEXT_OUT" \
  "add stripe webhook endpoint|add idempotency key to payments"

check_contains \
  "prompt: what did we decide about payments?" \
  "context includes Decisions section and does not return 404-style error" \
  "$CONTEXT_OUT" \
  "## Decisions"
check_not_contains \
  "prompt: what did we decide about payments? (no 404)" \
  "context should not contain 404" \
  "$CONTEXT_OUT" \
  "404"

check_contains \
  "prompt: where did we leave off?" \
  "context includes goal and task state sections" \
  "$CONTEXT_OUT" \
  "## Active goals"
check_contains \
  "prompt: where did we leave off? (goal title)" \
  "context includes payment goal title" \
  "$CONTEXT_OUT" \
  "build a payment API"

run_in_dir "$TMP_PROJECT" "$IMI_BIN" task "$GOAL_ID" "add retry logic to webhooks" "retry transient webhook delivery failures safely"
check_contains \
  "prompt: add retry logic to webhooks to the backlog (create)" \
  "imi task exits 0" \
  "exit=$CMD_EXIT output=$CMD_OUT" \
  "exit=0"

run_in_dir "$TMP_PROJECT" "$IMI_BIN" status
STATUS_OUT="$CMD_OUT"
check_contains \
  "prompt: add retry logic to webhooks to the backlog (verify)" \
  "new backlog task appears in status output" \
  "$STATUS_OUT" \
  "add retry logic to webhooks"

echo ""
echo "--- Status after backlog add ---"
echo "$STATUS_OUT"
echo "--------------------------------"
echo ""

TOTAL=$((PASS + FAIL))
echo "$PASS/$TOTAL checks passed"
[[ "$FAIL" -eq 0 ]]
