FROM rust:1.83.0-bookworm AS builder
LABEL authors="Devin Stompanato"

WORKDIR /build
COPY . .

RUN cargo build --release

FROM docker.io/debian:bookworm-slim AS runner

WORKDIR /app

COPY --from=builder /build/target/release/shelly_smartplug_exporter .
RUN apt-get update \
    && apt-get upgrade \
    && apt-get install -y openssl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

USER 1001

ENTRYPOINT [ "./shelly_smartplug_exporter" ]
