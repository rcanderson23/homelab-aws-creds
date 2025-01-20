FROM --platform=$TARGETPLATFORM rust:1.84-bullseye AS builder

RUN apt-get update && apt-get -y install ca-certificates && update-ca-certificates
WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY . .

RUN cargo build --release

FROM --platform=$TARGETPLATFORM gcr.io/distroless/cc-debian12

WORKDIR /app

COPY --from=builder /etc/ssl/certs /etc/ssl/certs
COPY --from=builder /app/target/release/homelab-aws-creds /app/homelab-aws-creds

ENTRYPOINT [ "/app/homelab-aws-creds" ]

