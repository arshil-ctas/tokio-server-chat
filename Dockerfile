FROM rust:1-slim AS build
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p chatserver

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/chatserver /usr/local/bin/chatserver
EXPOSE 8080
CMD ["chatserver"]
