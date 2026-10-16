# Build Commands

All commands below are run from the repository root via `make <target>`.

| Command | Description |
| --- | --- |
| `make dev` | Runs the app in development mode with hot-reload |
| `make test` | Runs all local tests: the Rust backend suite followed by the frontend suite |
| `make test-backend` | Runs only the Rust backend unit/contract test suite |
| `make test-frontend` | Runs only the frontend `vitest` suite |
| `make test-docker` | Runs the **complete** backend test suite inside a disposable Docker Compose stack |
| `make lint` | Runs `cargo clippy` (backend) and `biome lint` (frontend) |
| `make format` | Formats backend code with `cargo fmt` and frontend code with `biome format`. |
| `make build-ubuntu` | Builds `.deb` and `.AppImage` release artifacts for Ubuntu |
| `make build-macos` | Builds `.dmg` and `.app` release artifacts for macOS |
| `make docker-build` | Builds the Docker image used by `make test-docker` and `make docker-dev` |
| `make docker-dev` | Opens an interactive shell inside the same Docker image, with the repository bind-mounted, for isolated manual development/debugging. |
| `make clean` | Removes build output: `dist/`, `src-tauri/target/`, and `node_modules/`. |

See [development](DEVELOPMENT.md) for project structure and architecture conventions.
