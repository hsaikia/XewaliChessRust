# Deploying XewaliChess Bot on Lichess

This guide covers deploying the Xewali chess engine as a 24/7 Lichess bot using Docker.

## Prerequisites

- A VPS with Docker and Docker Compose installed
- A [Lichess BOT account](https://lichess.org/api#tag/Bot)

## 1. Get a Lichess API Token

1. Log into lichess.org with your bot account.
2. Go to https://lichess.org/account/oauth/token/create
3. Select the `bot:play` scope. This is the only scope needed — it covers playing moves, accepting challenges, and chat.
4. Generate the token and save it.

> If your account is still a normal account, upgrade it to a BOT account first.
> This is a **one-time, irreversible** action and the account must not have played any games:
> ```bash
> curl -d '' https://lichess.org/api/bot/account/upgrade \
>   -H "Authorization: Bearer YOUR_TOKEN"
> ```

## 2. Deploy on VPS

The Docker image is built automatically by GitHub Actions on every push to `main` and published to GHCR. No compilation happens on the VPS.

Clone the repo and configure:

```bash
git clone https://github.com/hsaikia/XewaliChessRust.git
cd XewaliChessRust
cp .env.example .env
```

Edit `.env` and paste your token:

```
LICHESS_BOT_TOKEN=lip_xxxxxxxxxxxxxxxxxxxxxxxx
```

Pull and run:

```bash
docker compose pull
docker compose up -d
```

Verify the bot is connected:

```bash
docker compose logs -f
```

You should see output indicating the bot has connected to Lichess and is waiting for challenges.

## 3. Updating

After pushing changes to `main`, GitHub Actions will build and push a new image. On your VPS:

```bash
docker compose pull
docker compose up -d
```

## 4. Managing the Bot

| Action | Command |
|---|---|
| Start | `docker compose up -d` |
| Stop | `docker compose down` |
| View live logs | `docker compose logs -f` |
| Restart | `docker compose restart` |
| Update to latest image | `docker compose pull && docker compose up -d` |

The `restart: unless-stopped` policy ensures the bot automatically restarts after crashes or VPS reboots.

## Configuration

Edit `config.yml` to adjust bot behavior. Key settings:

| Setting | Default | Description |
|---|---|---|
| `challenge.concurrency` | `1` | Number of simultaneous games |
| `challenge.time_controls` | bullet, blitz, rapid, classical | Accepted time controls |
| `challenge.variants` | standard, chess960 | Accepted variants |
| `challenge.modes` | casual, rated | Accepted game modes |
| `challenge.accept_bot` | `false` | Accept challenges from other bots |
| `challenge.min_base` | `30` | Minimum initial time (seconds) |
| `challenge.max_base` | `10800` | Maximum initial time (seconds) |
| `challenge.min_increment` | `0` | Minimum increment (seconds) |
| `challenge.max_increment` | `180` | Maximum increment (seconds) |

The engine dynamically allocates thinking time based on the remaining clock and increment, so it adapts to any time control from bullet to classical.

After editing `config.yml`, push to `main` and redeploy:

```bash
git push
# Wait for GitHub Actions to finish, then on VPS:
docker compose pull && docker compose up -d
```

See the [lichess-bot configuration docs](https://github.com/lichess-bot-devs/lichess-bot/wiki/Configure-lichess-bot) for the full list of options.
