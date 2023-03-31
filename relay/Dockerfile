FROM rust:1-slim-buster AS build
WORKDIR /app
COPY . .
RUN cargo build --release -p relay

FROM debian:buster-slim

RUN set -ex \
    \
    && apt-get update \
    && apt-get install -y ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
RUN update-ca-certificates

WORKDIR /app
COPY --from=build /app/target/release/relay /app/

ENV RUST_LOG=info
EXPOSE 80
ENTRYPOINT ["/app/relay"]
