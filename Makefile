.PHONY: all help setup dev test test-frontend test-backend build-ubuntu build-macos clean lint format

SHELL := /bin/bash

all: help

help:
	@echo "NebuLima Control Center - Build & Development Tasks"
	@echo ""
	@echo "Local Commands:"
	@echo "  make setup           Install npm deps and the pinned Rust toolchain"
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

# Environment setup
setup:
	@command -v rustup >/dev/null 2>&1 || { echo "rustup not found - install it from https://rustup.rs, then re-run make setup"; exit 1; }
	rustup show
	npm install

# Development mode
dev:
	npm run dev

# Testing targets
test: test-backend test-frontend

test-backend:
	cd src-tauri && cargo test

test-frontend:
	npm test

# Platform release packaging targets. The colima/docker dependency check runs inside the
# app itself at first launch (src-tauri/src/preflight.rs), not here - these targets only
# build and bundle NebuLima, so build/CI machines never need colima or docker installed.
build-ubuntu:
	scripts/check-platform.sh linux
	npm run build
	cd src-tauri && npx tauri build --bundles deb,appimage

build-macos:
	scripts/check-platform.sh macos
	npm run build
	cd src-tauri && npx tauri build --bundles dmg,app

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
