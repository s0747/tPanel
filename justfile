# JUSTFILE
# cargo install just
# 
default:
    @just --list

# cargo install cargo-watch
watch:
    cargo watch -c -x "check"

[parallel]
watch-dev: watch-broker watch-panel watch-core

watch-broker:
    cd broker && cargo watch -x "run --bin broker"

watch-panel:
    cd panel && cargo watch -x "run --bin panel"

watch-core:
    cd core && cargo watch -x "run --bin core"

[parallel]
run: run-broker run-panel run-core

run-broker:
    cd broker && cargo run --bin broker

run-panel:
    cd panel && cargo run --bin panel

run-core:
    cd core && cargo run --bin core

build:
    cargo build --workspace

clean:
    cargo clean --workspace

check:
    just fix
    just deny
    just test
    just coverage
    just machete
    just doc

prepush:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test
    cargo deny check

fix:
    cargo fmt
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged
    cargo fmt
    cargo clippy --all-targets --all-features -- -D warnings

deny:
    cargo deny check

test:
    cargo test

# rustup component add llvm-tools-preview
# cargo install cargo-llvm-cov

# cargo install --locked cargo-tarpaulin
coverage:
    cargo tarpaulin --out Html --output-dir coverage

# cargo install cargo-machete
# Check unused dependencies
machete:
    cargo machete || true

# Build documentation
doc:
    cargo doc --no-deps --open

# LOCAL DOCKER DEV
# Konfiguracja lokalna
project_name := "tpnel-local-dev"
build_tag := "local-dev"
#db_path := "local_data"

db_path := justfile_directory() + "/local_data"

docker-build:
    docker build -f docker/Dockerfile -t {{ project_name }}:{{ build_tag }} .

docker-run:
    @mkdir -p {{ db_path }}
    @echo "Uruchamianie... DB DIR: {{ justfile_directory() }}/{{ db_path }}"
    docker run --rm -it \
        -p 8080:8080 \
        -v $(pwd)/{{ db_path }}:/app/data \
        --name {{ project_name }}-dev \
        {{ project_name }}:{{ build_tag }}

docker-stop:
    docker stop {{ project_name }}-dev || true

#docker-dev: stop build r

docker-clean:
    cargo clean
    docker rmi {{ project_name }}:{{ build_tag }} || true

docker-debug:
    docker exec -it {{ project_name }}-dev sh

# OLD LOCAL DEV

# jaeger:
#     docker run --rm --name xxx-jaeger -p 16686:16686 -p 4317:4317 -e COLLECTOR_OTLP_ENABLED=true jaegertracing/all-in-one:latest

# backend:
#     sh -c 'i=0; while [ "$i" -lt 60 ]; do docker info >/dev/null 2>&1 && exit 0; i=$((i+1)); sleep 1; done; echo "Docker not ready" >&2; exit 1'
#     docker compose -f compose.yml up -d postgres
#     docker compose -f compose.yml ps
#     cargo run --bin core

# jaeger-stop:
#     docker stop jaeger || true

# docker-up:
#     docker compose -f compose.yml up -d

# postgres-up:
#     docker compose -f compose.yml up -d postgres

# docker-ps:
#     docker compose -f compose.yml ps

# docker-logs service="postgres":
#     docker compose -f compose.yml logs -f {{ service }}

# docker-down:
#     docker compose -f compose.yml down

# docker-reset:
#     docker compose -f compose.yml down -v

# # LOCAL DEV
# # CRC-32 (X-CRC32 header) testing for POST /telemetry

# telemetry-ingest-no-crc file="body.json":
#     curl -sS -i -X POST http://127.0.0.1:3000/telemetry -H "content-type: application/json" --data-binary @"{{ file }}"

# telemetry-ingest-crc-ok file="body.json":
#     curl -sS -i -X POST http://127.0.0.1:3000/telemetry -H 'content-type: application/json' -H "x-crc32: $(just crc32 {{ file }})" --data-binary @"{{ file }}"

# telemetry-ingest-crc-bad file="body.json":
#     curl -sS -i -X POST http://127.0.0.1:3000/telemetry -H "content-type: application/json" -H "x-crc32: 00000000" --data-binary @"{{ file }}"

# telemetry-ingest-crc-invalid file="body.json":
#     curl -sS -i -X POST http://127.0.0.1:3000/telemetry -H "content-type: application/json" -H "x-crc32: not-hex" --data-binary @"{{ file }}"
