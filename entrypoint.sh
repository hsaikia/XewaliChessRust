#!/bin/sh
set -e

if [ -z "$LICHESS_BOT_TOKEN" ]; then
  echo "Error: LICHESS_BOT_TOKEN environment variable is not set"
  exit 1
fi

sed -i "s|__LICHESS_BOT_TOKEN__|${LICHESS_BOT_TOKEN}|g" config.yml

# Self-triggering: re-trigger this workflow before the 6h GitHub Actions timeout.
# This keeps the bot running 24/7 with only a brief gap during handoff.
# SELF_TRIGGER_TOKEN must be a GitHub PAT with Actions read/write (fine-grained)
# or workflow scope (classic) on this repo.
HANDOFF_SECONDS=19800  # 5h 30m — leaves a 30m buffer to retry if triggering fails
HANDOFF_FLAG=/tmp/handoff.done
REF="${REF:-main}"

trigger_next_run() {
  curl -s -o /dev/null -w "%{http_code}" \
    --connect-timeout 15 \
    --max-time 60 \
    --retry 3 \
    --retry-delay 5 \
    --retry-all-errors \
    -X POST \
    -H "Authorization: Bearer ${SELF_TRIGGER_TOKEN}" \
    -H "Accept: application/vnd.github+json" \
    -H "Content-Type: application/json" \
    "https://api.github.com/repos/${REPO}/actions/workflows/${WORKFLOW_FILE}/dispatches" \
    -d "{\"ref\":\"${REF}\"}"
}

echo "Starting lichess-bot..."
python3 lichess-bot.py &
BOT_PID=$!

if [ -n "$SELF_TRIGGER_TOKEN" ] && [ -n "$WORKFLOW_FILE" ] && [ -n "$REPO" ]; then
  echo "Self-triggering enabled. Will hand off in ${HANDOFF_SECONDS}s ($(( HANDOFF_SECONDS / 3600 ))h $(( (HANDOFF_SECONDS % 3600) / 60 ))m)."

  # Background process: sleeps, triggers the next run, then stops this bot.
  # The bot runs as a child of this shell (not PID 1), so SIGTERM works.
  (
    sleep "$HANDOFF_SECONDS"

    echo "$(date -u): Handoff window reached. Triggering next workflow run..."
    while :; do
      HTTP_CODE=$(trigger_next_run) || HTTP_CODE=000

      if [ "$HTTP_CODE" = "204" ]; then
        echo "$(date -u): Next run triggered successfully (HTTP $HTTP_CODE). Handing off."
        break
      fi

      echo "$(date -u): Trigger failed (HTTP $HTTP_CODE). Retrying in 60s..."
      sleep 60
    done

    # Mark the handoff as done before stopping the bot so the parent shell
    # always sees the flag when `wait` returns.
    touch "$HANDOFF_FLAG"
    kill -TERM "$BOT_PID" 2>/dev/null || true
  ) &
fi

# Wait for the bot to exit — either on its own (crash) or after a successful handoff.
set +e
wait "$BOT_PID"
BOT_EXIT=$?
set -e

if [ -f "$HANDOFF_FLAG" ]; then
  echo "$(date -u): Handoff complete. Exiting cleanly."
  exit 0
fi

echo "$(date -u): Bot exited unexpectedly (code $BOT_EXIT)."
exit "$BOT_EXIT"
