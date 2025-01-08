name := "homelab-aws-creds"

default:
  @just --list

fmt:
  cargo fmt

container:
  docker buildx build --tag {{name}}:latest . --load

build: 
  cargo build --release

serve:
  cargo run --release

