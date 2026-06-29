#!/bin/sh
set -e

if [ -z "$LICHESS_BOT_TOKEN" ]; then
  echo "Error: LICHESS_BOT_TOKEN environment variable is not set"
  exit 1
fi

sed -i "s|__LICHESS_BOT_TOKEN__|${LICHESS_BOT_TOKEN}|g" config.yml

# Self-triggering: re-trigger this workflow ~5 minutes before the 6h GitHub Actions timeout.
# This keeps the bot running 24/7 with only a brief gap during handoff.
# SELF_TRIGGER_TOKEN must be a GitHub PAT (classic) with "workflow" scope on this repo.
HANDOFF_SECONDS=20700  # 5h 45m

if [ -n "$SELF_TRIGGER_TOKEN" ] && [ -n "$WORKFLOW_FILE" ] && [ -n "$REPO" ]; then
  echo "Self-triggering enabled. Will hand off in ${HANDOFF_SECONDS}s ($(( HANDOFF_SECONDS / 3600 ))h $(( (HANDOFF_SECONDS % 3600) / 60 ))m)."

  # Background process: sleeps, triggers next run, then signals this process to exit.
  (
    sleep "$HANDOFF_SECONDS"

    echo "$(date -u): Triggering next workflow run..."
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
      -X POST \
      -H "Authorization: Bearer ${SELF_TRIGGER_TOKEN}" \
      -H "Accept: application/vnd.github+json" \
      "https://api.github.com/repos/${REPO}/actions/workflows/${WORKFLOW_FILE}/dispatches" \
      -d "{\"ref\":\"main\"}")

    if [ "$HTTP_CODE" = "204" ]; then
      echo "$(date -u): Next run triggered successfully (HTTP $HTTP_CODE). Exiting."
    else
      echo "$(date -u): Trigger failed (HTTP $HTTP_CODE). Exiting anyway — bot will be offline until manual restart."
    fi

    # Signal the main process to exit.
    # We send SIGTERM to PID 1 (the shell running this script, replaced by python via exec).
    kill -TERM 1 2>/dev/null || true
  ) &
fi

echo "Starting lichess-bot..."
exec python3 lichess-bot.py
