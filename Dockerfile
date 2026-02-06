# Stage 1: Build the Rust engine
FROM rust:1.84-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

# Stage 2: Runtime with lichess-bot
FROM python:3.12-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends git \
    && rm -rf /var/lib/apt/lists/*

RUN git clone --depth 1 https://github.com/lichess-bot-devs/lichess-bot.git /app \
    && pip install --no-cache-dir -r requirements.txt

RUN mkdir -p engines
COPY --from=builder /build/target/release/xewali_engine engines/
RUN chmod +x engines/xewali_engine

# Smoke test: verify engine responds to UCI
RUN echo "uci" | engines/xewali_engine | head -1 | grep -q "id name"

COPY config.yml config.yml
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
