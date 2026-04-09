FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
RUN apt-get update \
	&& apt-get install -y --no-install-recommends pkg-config libudev-dev libdbus-1-dev \
	&& rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY src ./src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY src ./src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates libdbus-1-3 libudev1 \
	&& rm -rf /var/lib/apt/lists/*

ARG BINARY_NAME=ble_stream
COPY --from=builder /app/target/release/${BINARY_NAME} /usr/local/bin/${BINARY_NAME}

EXPOSE 8080
CMD ["/usr/local/bin/ble_stream"]
