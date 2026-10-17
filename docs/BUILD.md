# Build Commands

All commands below are run from the repository root via `make <target>`.

| Command | Description |
| --- | --- |
| `make setup` | Install npm deps and the pinned Rust toolchain. Requires `rustup` itself to already be installed ([rustup.rs](https://rustup.rs)). |
| `make dev` | Runs the app in development mode with hot-reload |
| `make test` | Runs all tests. This is what CI runs |
| `make test-backend` | Runs only the Rust backend unit/contract test suite |
| `make test-frontend` | Runs only the frontend `vitest` suite |
| `make lint` | Runs `cargo clippy` (backend) and `biome lint` (frontend) |
| `make format` | Formats backend code with `cargo fmt` and frontend code with `biome format`. |
| `make build-ubuntu` | Builds `.deb` and `.AppImage` release artifacts for Ubuntu. Does not run on a non-Linux host. |
| `make build-macos` | Builds `.dmg` and `.app` release artifacts for macOS. Does not run on a non-macOS host. |
| `make clean` | Removes build output: `dist/`, `src-tauri/target/`, and `node_modules/`. |

See [development](DEVELOPMENT.md) for project structure and architecture conventions.
