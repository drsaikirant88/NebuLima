.PHONY: all help dev test test-frontend test-backend test-docker build-ubuntu build-macos docker-build docker-dev clean lint format

SHELL := /bin/bash

all: help

help:
	@echo "NebuLima Control Center - Build & Development Tasks"
	@echo ""
	@echo "Local Commands:"
	@echo "  make dev             Run app in development mode with hot-reload"
	@echo "  make test            Run all local tests (cargo test + vitest)"
	@echo "  make test-backend    Run Rust backend unit/contract tests"
	@echo "  make test-frontend   Run frontend tests (vitest)"
	@echo "  make lint            Run linter checks for Rust and frontend"
	@echo "  make format          Format frontend and backend code"
	@echo ""
	@echo "Packaging Commands:"
	@echo "  make build-ubuntu    Build release artifacts for Ubuntu (.deb, .AppImage)"
	@echo "  make build-macos     Build release artifacts for macOS (.dmg, .app)"
	@echo ""
	@echo "Docker-based Isolated Development & Testing:"
	@echo "  make docker-build    Build Docker test and development image"
	@echo "  make test-docker     Run complete backend test suite inside Docker container"
	@echo "  make docker-dev      Run interactive development container shell"
	@echo ""

# Development mode
dev:
	npm run dev

# Testing targets
test: test-backend test-frontend

test-backend:
	cd src-tauri && cargo test

test-frontend:
	npm test

# Docker-isolated testing
test-docker:
	@echo "Running full backend test suite in Docker container..."
	docker compose -f tests/docker/docker-compose.yml up --build --abort-on-container-exit --exit-code-from test-runner
	@docker compose -f tests/docker/docker-compose.yml down -v

# Interactive Docker dev environment
docker-build:
	docker build -t nebulima-dev -f tests/docker/Dockerfile .

docker-dev: docker-build
	docker run -it --rm \
		-v "$(PWD):/workspace" \
		-w /workspace \
		nebulima-dev /bin/bash

# Platform release packaging targets. The colima/docker dependency check runs inside the
# app itself at first launch (src-tauri/src/preflight.rs), not here - these targets only
# build and bundle NebuLima, so build/CI machines never need colima or docker installed.
build-ubuntu:
	npm run build
	cd src-tauri && cargo tauri build --bundles deb,appimage

build-macos:
	npm run build
	cd src-tauri && cargo tauri build --bundles dmg,app

# Code hygiene
lint:
	cd src-tauri && cargo clippy --all-targets -- -D warnings
	npm run lint

format:
	cd src-tauri && cargo fmt
	npm run format

clean:
	rm -rf dist
	rm -rf src-tauri/target
	rm -rf node_modules

