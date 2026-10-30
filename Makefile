.PHONY: all help setup dev test test-frontend test-backend test-scripts build-ubuntu build-macos release clean lint format

SHELL := /bin/bash

all: help

help:
	@echo "NebuLima Control Center - Build & Development Tasks"
	@echo ""
	@echo "Local Commands:"
	@echo "  make setup           Install npm deps and the pinned Rust toolchain"
	@echo "  make dev             Run app in development mode with hot-reload"
	@echo "  make test            Run all local tests (cargo test + vitest + shell scripts)"
	@echo "  make test-backend    Run Rust backend unit/contract tests"
	@echo "  make test-frontend   Run frontend tests (vitest)"
	@echo "  make test-scripts    Run scripts/check-platform.test.sh"
	@echo "  make lint            Run linter checks for Rust and frontend"
	@echo "  make format          Format frontend and backend code"
	@echo ""
	@echo "Packaging Commands:"
	@echo "  make build-ubuntu    Build release artifacts for Ubuntu (.deb, .AppImage)"
	@echo "  make build-macos     Build release artifacts for macOS (.dmg, .app)"
	@echo "  make release VERSION=X.Y.Z   Tag and push a release (run on main, after the"
	@echo "                                version-bump PR has merged)"
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
test: test-backend test-frontend test-scripts

test-backend:
	cd src-tauri && cargo test

test-frontend:
	npm test

test-scripts:
	scripts/check-platform.test.sh

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

# Tags and pushes a release, triggering .github/workflows/release.yml. Run this on main,
# only after the version-bump PR (docs/CONTRIBUTING.md#releasing-a-new-version) has already
# merged - never from a PR branch, since a squash/rebase merge rewrites commit SHAs and would
# leave a pre-merge tag pointing at a commit that never actually lands on main.
release:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make release VERSION=X.Y.Z"; \
		exit 1; \
	fi
	@branch=$$(git rev-parse --abbrev-ref HEAD); \
	if [ "$$branch" != "main" ]; then \
		echo "Must be run on main (currently on '$$branch'). Merge the version-bump PR to main first."; \
		exit 1; \
	fi
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "Working tree is not clean. Commit or stash changes first."; \
		exit 1; \
	fi
	@git fetch origin main; \
	if [ "$$(git rev-parse HEAD)" != "$$(git rev-parse origin/main)" ]; then \
		echo "Local main is not up to date with origin/main. Run 'git pull' first."; \
		exit 1; \
	fi
	@pkg=$$(node -p "require('./package.json').version"); \
	tauri=$$(node -p "require('./src-tauri/tauri.conf.json').version"); \
	cargo=$$(grep -m1 '^version' src-tauri/Cargo.toml | sed -E 's/version = "(.*)"/\1/'); \
	if [ "$$pkg" != "$(VERSION)" ] || [ "$$tauri" != "$(VERSION)" ] || [ "$$cargo" != "$(VERSION)" ]; then \
		echo "Version mismatch: package.json=$$pkg tauri.conf.json=$$tauri Cargo.toml=$$cargo (expected $(VERSION))."; \
		echo "Bump all three to $(VERSION) via a PR and merge it to main before releasing."; \
		exit 1; \
	fi
	@if git rev-parse "v$(VERSION)" >/dev/null 2>&1 || git ls-remote --exit-code --tags origin "refs/tags/v$(VERSION)" >/dev/null 2>&1; then \
		echo "Tag v$(VERSION) already exists (locally or on origin)."; \
		exit 1; \
	fi
	git tag -a "v$(VERSION)" -m "NebuLima v$(VERSION)"
	git push origin "v$(VERSION)"
	@echo "Pushed tag v$(VERSION) - release.yml will build artifacts into a draft GitHub release. Publish it manually from the Releases tab once it's ready."
