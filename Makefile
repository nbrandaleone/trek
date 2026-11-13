PORT ?= 8080
IMAGE_NAME ?= trek
CONTAINER_NAME ?= trek

.PHONY: all build wasm start run docker docker-build docker-run docker-start docker-stop test unit-test check clean help

all: build

build: wasm
	cargo build --release --bin trek

wasm:
	sh scripts/build_web.sh

start: wasm
	cargo run --release --bin trek -- --port $(PORT)

run: start

docker-build:
	docker build -t $(IMAGE_NAME) .

docker-run: docker-build
	docker run --rm --name $(CONTAINER_NAME) -p $(PORT):$(PORT) -e PORT=$(PORT) $(IMAGE_NAME)

docker-start: docker-run

docker: docker-run

docker-stop:
	docker stop $(CONTAINER_NAME)

test:
	cargo test --no-fail-fast

unit-test: test

check:
	cargo check
	cargo check --lib --target wasm32-unknown-unknown
	node --check web/app.js

clean:
	cargo clean
	rm -f web/trek.wasm

help:
	@echo "Trek (Super Star Trek for Mac OS -> Rust + WebAssembly) Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  make all | build        Build web/trek.wasm and release server binary (trek)"
	@echo "  make wasm               Build web/trek.wasm via scripts/build_web.sh"
	@echo "  make start | run        Build WASM and start the web server on port $(PORT)"
	@echo "                          (override port with: make start PORT=3000)"
	@echo "  make docker | docker-run Build image and run container on port $(PORT)"
	@echo "                          (override port with: make docker-run PORT=3000)"
	@echo "  make docker-build       Build the $(IMAGE_NAME) Docker image"
	@echo "  make docker-stop        Stop the running $(CONTAINER_NAME) Docker container"
	@echo "  make test | unit-test   Run the full Cargo unit and integration test suite"
	@echo "  make check              Type-check native, wasm32-unknown-unknown, and web/app.js"
	@echo "  make clean              Remove Cargo build artifacts and web/trek.wasm"
	@echo "  make help               Show this help message"
