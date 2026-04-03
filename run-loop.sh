#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Implementation Loop Runner
#
# Runs the implementation agent in a loop until all components are DONE,
# a component is BLOCKED, or a safety limit is hit.
#
# Usage:
#   ./run-loop.sh [--max-iterations N] [--spec PATH] [--project-root PATH]
#
# Prerequisites:
#   - opencode CLI installed and configured
#   - spec file exists at the specified path
#   - implement.md prompt exists in the prompts directory
# =============================================================================

# --- Configuration -----------------------------------------------------------

MAX_ITERATIONS=50          # Safety valve — no project needs 50 iterations
SPEC_PATH="spec/spec.md"  # Override with --spec
PROJECT_ROOT="."           # Override with --project-root
STATE_FILE="state.md"
LOG_DIR=".loop-logs"
PROMPT_FILE="prompts/implement.md"

# How to invoke the agent. Adjust this for your setup.
# OpenCode: opencode run "message" --file prompt.md
# Claude Code: claude -p "prompt" --model opus
AGENT_CMD="opencode"
AGENT_SUBCMD="run"
AGENT_FLAGS=""  # e.g., "-m provider/model" to pin a model

# Cooldown between iterations (seconds). Prevents runaway API spend if
# the agent starts exiting immediately due to a bug.
COOLDOWN=5

# --- Parse arguments ---------------------------------------------------------

while [[ $# -gt 0 ]]; do
  case $1 in
    --max-iterations) MAX_ITERATIONS="$2"; shift 2 ;;
    --spec)           SPEC_PATH="$2"; shift 2 ;;
    --project-root)   PROJECT_ROOT="$2"; shift 2 ;;
    --prompt)         PROMPT_FILE="$2"; shift 2 ;;
    --cooldown)       COOLDOWN="$2"; shift 2 ;;
    --agent-cmd)      AGENT_CMD="$2"; shift 2 ;;
    --agent-subcmd)   AGENT_SUBCMD="$2"; shift 2 ;;
    --agent-flags)    AGENT_FLAGS="$2"; shift 2 ;;
    *)                echo "Unknown option: $1"; exit 1 ;;
  esac
done

# --- Resolve project root and cd into it -------------------------------------

PROJECT_ROOT="$(realpath "$PROJECT_ROOT")"
if [[ ! -d "$PROJECT_ROOT" ]]; then
  echo "❌ Project root not found: $PROJECT_ROOT"
  exit 1
fi
cd "$PROJECT_ROOT"

# Resolve remaining paths relative to project root
SPEC_PATH="$(realpath "$SPEC_PATH" 2>/dev/null || echo "$SPEC_PATH")"
PROMPT_FILE="$(realpath "$PROMPT_FILE" 2>/dev/null || echo "$PROMPT_FILE")"
LOG_DIR="$PROJECT_ROOT/$LOG_DIR"

# --- Preflight checks --------------------------------------------------------

if [[ ! -f "$SPEC_PATH" ]]; then
  echo "❌ Spec not found at $SPEC_PATH"
  exit 1
fi

if [[ ! -f "$PROMPT_FILE" ]]; then
  echo "❌ Prompt not found at $PROMPT_FILE"
  exit 1
fi

if ! command -v "$AGENT_CMD" &> /dev/null; then
  echo "❌ Agent command '$AGENT_CMD' not found in PATH"
  exit 1
fi

mkdir -p "$LOG_DIR"

# --- State file parsing ------------------------------------------------------

get_iteration_number() {
  if [[ ! -f "$STATE_FILE" ]]; then
    echo "0"
    return
  fi
  # Extract current iteration from state file
  local n
  n=$(grep -oP '(?<=\*\*Current iteration\*\*: )\d+' "$STATE_FILE" 2>/dev/null || echo "0")
  echo "$n"
}

count_by_status() {
  local status="$1"
  if [[ ! -f "$STATE_FILE" ]]; then
    echo "0"
    return
  fi
  grep -c "| ${status} |" "$STATE_FILE" 2>/dev/null || echo "0"
}

has_blocked() {
  if [[ ! -f "$STATE_FILE" ]]; then
    echo "false"
    return
  fi
  if grep -q "| BLOCKED |" "$STATE_FILE" 2>/dev/null; then
    echo "true"
  else
    echo "false"
  fi
}

has_in_progress() {
  if [[ ! -f "$STATE_FILE" ]]; then
    echo "false"
    return
  fi
  if grep -q "| IN_PROGRESS |" "$STATE_FILE" 2>/dev/null; then
    echo "true"
  else
    echo "false"
  fi
}

has_remaining_work() {
  if [[ ! -f "$STATE_FILE" ]]; then
    # No state file = first iteration, there's work to do
    echo "true"
    return
  fi
  local not_started in_progress
  not_started=$(count_by_status "NOT_STARTED")
  in_progress=$(count_by_status "IN_PROGRESS")
  if [[ "$not_started" -gt 0 || "$in_progress" -gt 0 ]]; then
    echo "true"
  else
    echo "false"
  fi
}

count_deviations() {
  if [[ ! -f "$STATE_FILE" ]]; then
    echo "0"
    return
  fi
  grep -c "^### DEV-" "$STATE_FILE" 2>/dev/null || echo "0"
}

# --- Build the per-iteration message -----------------------------------------

build_message() {
  local iteration="$1"
  if [[ ! -f "$STATE_FILE" ]]; then
    cat <<EOF
The attached file contains your implementation loop instructions. Follow them exactly.

This is iteration $iteration (first run — bootstrap).
Spec location: $SPEC_PATH
State file: $STATE_FILE
No state file exists yet. Execute the Bootstrap task as described in the attached instructions.
EOF
  else
    cat <<EOF
The attached file contains your implementation loop instructions. Follow them exactly.

This is iteration $iteration.
Spec location: $SPEC_PATH
State file: $STATE_FILE
Read state.md FIRST, then execute exactly one iteration of the implementation loop.
EOF
  fi
}

# --- Status display ----------------------------------------------------------

print_status() {
  local iteration="$1"
  local done not_started blocked in_progress deviations
  done=$(count_by_status "DONE")
  not_started=$(count_by_status "NOT_STARTED")
  blocked=$(count_by_status "BLOCKED")
  in_progress=$(count_by_status "IN_PROGRESS")
  deviations=$(count_deviations)

  echo ""
  echo "╔══════════════════════════════════════════╗"
  echo "║        Iteration $iteration complete              ║"
  echo "╠══════════════════════════════════════════╣"
  printf "║  ✅ Done:          %-20s ║\n" "$done"
  printf "║  🔄 In Progress:   %-20s ║\n" "$in_progress"
  printf "║  ⏳ Not Started:   %-20s ║\n" "$not_started"
  printf "║  🚫 Blocked:       %-20s ║\n" "$blocked"
  printf "║  📝 Deviations:    %-20s ║\n" "$deviations"
  echo "╚══════════════════════════════════════════╝"
  echo ""
}

# --- Stuck detection ---------------------------------------------------------

LAST_STATE_HASH=""

check_stuck() {
  if [[ ! -f "$STATE_FILE" ]]; then
    echo "false"
    return
  fi
  local current_hash
  current_hash=$(md5sum "$STATE_FILE" | cut -d' ' -f1)

  if [[ "$current_hash" == "$LAST_STATE_HASH" ]]; then
    echo "true"
  else
    LAST_STATE_HASH="$current_hash"
    echo "false"
  fi
}

STUCK_COUNT=0
MAX_STUCK=3  # Exit after 3 consecutive iterations with no state change

# --- Main loop ---------------------------------------------------------------

echo "🚀 Implementation loop starting"
echo "   Spec: $SPEC_PATH"
echo "   Agent: $AGENT_CMD $AGENT_SUBCMD $AGENT_FLAGS"
echo "   Max iterations: $MAX_ITERATIONS"
echo "   Cooldown: ${COOLDOWN}s"
echo "   Logs: $LOG_DIR/"
echo ""

iteration=0

while true; do
  iteration=$((iteration + 1))

  # --- Safety valve ---
  if [[ $iteration -gt $MAX_ITERATIONS ]]; then
    echo "🛑 Max iterations ($MAX_ITERATIONS) reached. Stopping."
    echo "   Review $STATE_FILE for progress."
    exit 2
  fi

  # --- Check if there's work remaining ---
  if [[ "$(has_remaining_work)" == "false" ]]; then
    echo "✅ All components DONE. Implementation complete."
    print_status "$((iteration - 1))"
    exit 0
  fi

  # --- Check for blocked components ---
  if [[ "$(has_blocked)" == "true" && "$(has_in_progress)" == "false" ]]; then
    # Only stop for blocked if there's no in-progress work to resume
    # and no unblocked NOT_STARTED components remain
    unblocked_remaining=$(count_by_status "NOT_STARTED")
    if [[ "$unblocked_remaining" -eq 0 ]]; then
      echo "🚫 All remaining components are BLOCKED. Human intervention needed."
      print_status "$((iteration - 1))"
      echo "   Review the BLOCKED section in $STATE_FILE"
      exit 3
    fi
  fi

  # --- Run the agent ---
  echo "▶ Iteration $iteration starting..."
  log_file="$LOG_DIR/iteration-$(printf '%03d' $iteration).log"
  message=$(build_message "$iteration")

  # Run agent: message first, then --file attaches implementation prompt as context
  set +e
  $AGENT_CMD $AGENT_SUBCMD $AGENT_FLAGS "$message" --file "$PROMPT_FILE" > "$log_file" 2>&1
  agent_exit=$?
  set -e

  if [[ $agent_exit -ne 0 ]]; then
    echo "⚠️  Agent exited with code $agent_exit. Log: $log_file"
    # Don't immediately fail — check if it made progress
  fi

  # --- Post-iteration checks ---
  print_status "$iteration"

  # Stuck detection: did the state file change?
  if [[ "$(check_stuck)" == "true" ]]; then
    STUCK_COUNT=$((STUCK_COUNT + 1))
    echo "⚠️  State unchanged after iteration $iteration (stuck count: $STUCK_COUNT/$MAX_STUCK)"
    if [[ $STUCK_COUNT -ge $MAX_STUCK ]]; then
      echo "🛑 Agent appears stuck — $MAX_STUCK iterations with no progress. Stopping."
      echo "   Last log: $log_file"
      exit 4
    fi
  else
    STUCK_COUNT=0
  fi

  # Cooldown
  if [[ "$(has_remaining_work)" == "true" ]]; then
    echo "   Next iteration in ${COOLDOWN}s... (Ctrl+C to pause)"
    sleep "$COOLDOWN"
  fi
done
