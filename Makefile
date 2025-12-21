# ═══════════════════════════════════════════════════════════════════════════
# rust-agent Makefile
# ═══════════════════════════════════════════════════════════════════════════

.PHONY: dev run build test clean setup help

# ─────────────────────────────────────────────────────────────────────────────
# Development
# ─────────────────────────────────────────────────────────────────────────────

## Start development server with auto-reload
dev:
	cargo watch -x 'run --bin agent-server'

## Run the server (no auto-reload)
run:
	cargo run --bin agent-server

## Run with release optimizations
run-release:
	cargo run --release --bin agent-server

# ─────────────────────────────────────────────────────────────────────────────
# Building
# ─────────────────────────────────────────────────────────────────────────────

## Build debug version
build:
	cargo build

## Build optimized release version
build-release:
	cargo build --release

## Build WASM frontend (requires trunk)
build-web:
	cd crates/agent-web && trunk build --release

# ─────────────────────────────────────────────────────────────────────────────
# Testing & Quality
# ─────────────────────────────────────────────────────────────────────────────

## Run all tests
test:
	cargo test --all

## Run tests with output
test-verbose:
	cargo test --all -- --nocapture

## Type check without building
check:
	cargo check --all

## Lint with clippy
lint:
	cargo clippy --all -- -D warnings

## Format code
fmt:
	cargo fmt --all

## Format check (CI)
fmt-check:
	cargo fmt --all -- --check

# ─────────────────────────────────────────────────────────────────────────────
# Stripe Testing
# ─────────────────────────────────────────────────────────────────────────────

## Forward Stripe webhooks to local server (requires Stripe CLI)
stripe-listen:
	stripe listen --forward-to localhost:3000/webhook/stripe

## Trigger a test webhook
stripe-trigger:
	stripe trigger checkout.session.completed

# ─────────────────────────────────────────────────────────────────────────────
# Setup & Utilities
# ─────────────────────────────────────────────────────────────────────────────

## Initial setup
setup:
	@echo "Setting up rust-agent..."
	@cp -n .env.example .env 2>/dev/null || true
	@echo "✓ Created .env from .env.example"
	@rustup target add wasm32-unknown-unknown
	@echo "✓ Added WASM target"
	@cargo install trunk cargo-watch
	@echo "✓ Installed trunk and cargo-watch"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Edit .env with your Stripe keys"
	@echo "  2. Start Ollama: ollama serve"
	@echo "  3. Pull a model: ollama pull llama3.2"
	@echo "  4. Run: make dev"

## Clean build artifacts
clean:
	cargo clean

## Show dependency tree
deps:
	cargo tree

## Update dependencies
update:
	cargo update

# ─────────────────────────────────────────────────────────────────────────────
# Docker
# ─────────────────────────────────────────────────────────────────────────────

## Build Docker image
docker-build:
	docker build -t rust-agent .

## Run in Docker
docker-run:
	docker run -p 3000:3000 --env-file .env rust-agent

# ─────────────────────────────────────────────────────────────────────────────
# Help
# ─────────────────────────────────────────────────────────────────────────────

## Show this help
help:
	@echo "rust-agent - Available commands:"
	@echo ""
	@echo "Development:"
	@echo "  make dev          - Start with auto-reload"
	@echo "  make run          - Start server"
	@echo "  make run-release  - Start optimized server"
	@echo ""
	@echo "Building:"
	@echo "  make build        - Build debug"
	@echo "  make build-release - Build release"
	@echo "  make build-web    - Build WASM frontend"
	@echo ""
	@echo "Testing:"
	@echo "  make test         - Run tests"
	@echo "  make lint         - Run clippy"
	@echo "  make fmt          - Format code"
	@echo ""
	@echo "Stripe:"
	@echo "  make stripe-listen  - Forward webhooks locally"
	@echo "  make stripe-trigger - Test webhook"
	@echo ""
	@echo "Setup:"
	@echo "  make setup        - Initial project setup"
	@echo "  make clean        - Clean build artifacts"
