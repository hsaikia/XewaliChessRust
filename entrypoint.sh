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
HANDOFF_SECONDS="${HANDOFF_SECONDS:-19800}" # 5h 30m — leaves a 30m buffer to retry if triggering fails
HANDOFF_FLAG=/tmp/handoff.done
BOT_PIDFILE=/tmp/bot.pid
REF="${REF:-main}"

# A bot that dies this fast never really came up; several in a row means
# something is broken that a restart will not fix.
FAST_CRASH_SECONDS=60
MAX_FAST_CRASHES=5

log() {
  echo "$(date -u '+%Y-%m-%dT%H:%M:%SZ') $*"
}

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

if [ -n "$SELF_TRIGGER_TOKEN" ] && [ -n "$WORKFLOW_FILE" ] && [ -n "$REPO" ]; then
  log "Self-triggering enabled. Will hand off in ${HANDOFF_SECONDS}s ($(( HANDOFF_SECONDS / 3600 ))h $(( (HANDOFF_SECONDS % 3600) / 60 ))m)."

  # Background process: sleeps, triggers the next run, then stops the bot.
  # It reads the PID file rather than capturing a PID, so it still stops the
  # right process if the supervisor has restarted the bot in the meantime.
  (
    sleep "$HANDOFF_SECONDS"

    log "Handoff window reached. Triggering next workflow run..."
    while :; do
      HTTP_CODE=$(trigger_next_run) || HTTP_CODE=000

      if [ "$HTTP_CODE" = "204" ]; then
        log "Next run triggered successfully (HTTP $HTTP_CODE). Handing off."
        break
      fi

      log "Trigger failed (HTTP $HTTP_CODE). Retrying in 60s..."
      sleep 60
    done

    # Mark the handoff as done before stopping the bot so the supervisor always
    # sees the flag when `wait` returns, and stops restarting.
    touch "$HANDOFF_FLAG"

    # Re-read the PID file each time: the supervisor may have restarted the bot
    # between the flag being set and the signal landing. Escalate if the bot
    # ignores SIGTERM, so a wedged process cannot hold the runner for 6h.
    attempt=0
    while :; do
      pid=$(cat "$BOT_PIDFILE" 2>/dev/null || echo "")
      if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
        break
      fi
      if [ "$attempt" -ge 30 ]; then
        log "Bot did not stop after SIGTERM. Sending SIGKILL to $pid."
        kill -KILL "$pid" 2>/dev/null || true
        break
      fi
      kill -TERM "$pid" 2>/dev/null || true
      attempt=$(( attempt + 1 ))
      sleep 1
    done
  ) &
else
  log "Self-triggering disabled (SELF_TRIGGER_TOKEN, WORKFLOW_FILE and REPO must all be set). Running until stopped."
fi

# Supervise the bot: a crash or a dropped Lichess stream restarts it in seconds
# instead of waiting for the keepalive watchdog to notice minutes later.
fast_crashes=0

while :; do
  log "Starting lichess-bot..."
  started_at=$(date -u +%s)

  python3 lichess-bot.py &
  BOT_PID=$!
  echo "$BOT_PID" > "$BOT_PIDFILE"

  # The handoff may have landed while this bot was starting, in which case the
  # signal went to the previous PID. Stop it now rather than holding the runner.
  if [ -f "$HANDOFF_FLAG" ]; then
    kill -TERM "$BOT_PID" 2>/dev/null || true
  fi

  set +e
  wait "$BOT_PID"
  BOT_EXIT=$?
  set -e

  if [ -f "$HANDOFF_FLAG" ]; then
    log "Handoff complete. Exiting cleanly."
    exit 0
  fi

  ran_for=$(( $(date -u +%s) - started_at ))
  log "lichess-bot exited unexpectedly (code $BOT_EXIT) after ${ran_for}s."

  if [ "$ran_for" -lt "$FAST_CRASH_SECONDS" ]; then
    fast_crashes=$(( fast_crashes + 1 ))
  else
    fast_crashes=0
  fi

  if [ "$fast_crashes" -ge "$MAX_FAST_CRASHES" ]; then
    log "Failed to stay up across $fast_crashes consecutive attempts. Exiting so the job fails and the watchdog starts a fresh run on a freshly pulled image."
    if [ "$BOT_EXIT" -eq 0 ]; then
      BOT_EXIT=1
    fi
    exit "$BOT_EXIT"
  fi

  backoff=$(( fast_crashes * 5 ))
  if [ "$backoff" -lt 5 ]; then
    backoff=5
  fi
  log "Restarting in ${backoff}s (consecutive fast crashes: ${fast_crashes})..."
  sleep "$backoff"

  # The handoff can land during the backoff; do not start a bot the successor
  # run is already replacing, or it would hold the runner until the 6h timeout.
  if [ -f "$HANDOFF_FLAG" ]; then
    log "Handoff completed while backing off. Exiting cleanly."
    exit 0
  fi
done
