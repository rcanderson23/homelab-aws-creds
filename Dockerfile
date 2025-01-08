FROM rust:1.82-bullseye AS builder

WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY . .

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

WORKDIR /app

COPY --from=builder /app/target/release/server /app/server

ENTRYPOINT [ "/app/server" ]

