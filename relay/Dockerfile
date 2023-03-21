FROM rust:1-slim-buster AS build
WORKDIR /app
COPY . .
RUN cargo build --release -p relay

FROM debian:buster-slim
WORKDIR /app
COPY --from=build /app/target/release/relay /app/

ENV RUST_LOG=info
EXPOSE 80
ENTRYPOINT ["/app/relay"]
