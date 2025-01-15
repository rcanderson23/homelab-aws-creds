name := "homelab-aws-creds"
dev_dir := "./dev"
config_dir := dev_dir + "/certs/config"
certs_dir := dev_dir + "/certs"
ca_dir := certs_dir + "/ca"

default:
  @just --list

fmt:
  cargo fmt

container:
  docker buildx build --tag {{name}}:latest . --load

run-container:
  docker run {{name}}:latest

build: 
  cargo build --release

serve-webhook:
  cargo run --release -- \
    webhook \
    --cert={{certs_dir}}/server/homelab-aws-creds-server.pem \
    --key={{certs_dir}}/server/homelab-aws-creds-server-key.pem \
    --role-mapping-path={{dev_dir}}/mappings.yaml

certs: certs-dir gen-ca gen-server

certs-dir:
  mkdir -p {{certs_dir}}/{ca,server}

gen-ca: gen-server-ca

gen-server-ca:
  cfssl gencert -initca {{config_dir}}/server-ca.json | cfssljson -bare {{name}}-server-ca
  mv {{name}}-server-ca* {{certs_dir}}/ca

gen-server:
  cfssl gencert \
    -ca={{ca_dir}}/{{name}}-server-ca.pem \
    -ca-key={{ca_dir}}/{{name}}-server-ca-key.pem \
    -config={{config_dir}}/ca-config.json \
    -profile=server \
     {{config_dir}}/server.json | cfssljson -bare {{name}}-server
  mv {{name}}-server* {{certs_dir}}/server
