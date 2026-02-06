#!/bin/sh
set -e

if [ -z "$LICHESS_BOT_TOKEN" ]; then
  echo "Error: LICHESS_BOT_TOKEN environment variable is not set"
  exit 1
fi

sed -i "s|__LICHESS_BOT_TOKEN__|${LICHESS_BOT_TOKEN}|g" config.yml

exec python3 lichess-bot.py
