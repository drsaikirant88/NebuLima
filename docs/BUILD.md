# Build Commands

## Node setup

NebuLima requires Node `>=26.13.0` (see `engines.node` in `package.json`). `.nvmrc` pins
the floating major version `26`, so with [nvm](https://github.com/nvm-sh/nvm) installed:

```sh
nvm install   # first time only, reads .nvmrc
nvm use       # reads .nvmrc
```

`.npmrc` sets `engine-strict=true`, so `npm ci`/`npm install` refuse to run on an older
Node instead of silently continuing. A local pre-commit hook
(`scripts/check-node-version.mjs`) checks the same floor before every commit.

## Make targets

All commands below are run from the repository root via `make <target>`.

| Command              | Description                                                                                                                                                                                                                            |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `make setup`         | Install npm deps and the pinned Rust toolchain. Requires `rustup` itself to already be installed ([rustup.rs](https://rustup.rs)).                                                                                                     |
| `make dev`           | Runs the app in development mode with hot-reload                                                                                                                                                                                       |
| `make test`          | Runs all tests: `cargo test`, `vitest`, and `scripts/check-platform.test.sh`. CI runs the equivalent checks split across the `Frontend` and `Backend` jobs, not this target directly (`make test` would otherwise run `vitest` twice). |
| `make test-backend`  | Runs only the Rust backend unit/contract test suite                                                                                                                                                                                    |
| `make test-frontend` | Runs only the frontend `vitest` suite                                                                                                                                                                                                  |
| `make test-scripts`  | Runs `scripts/check-platform.test.sh`                                                                                                                                                                                                  |
| `make lint`          | Runs `cargo clippy` (backend) and `biome lint` (frontend)                                                                                                                                                                              |
| `make format`        | Formats backend code with `cargo fmt` and frontend code with `biome format`.                                                                                                                                                           |
| `make build-ubuntu`  | Builds `.deb` and `.AppImage` release artifacts for Ubuntu, for the host's own architecture. Does not run on a non-Linux host.                                                                                                         |
| `make build-macos`   | Builds `.dmg` and `.app` release artifacts for macOS, for the host's own architecture. Does not run on a non-macOS host.                                                                                                               |
| `make clean`         | Removes build output: `dist/`, `src-tauri/target/`, and `node_modules/`.                                                                                                                                                               |

See [development](DEVELOPMENT.md) for project structure and architecture conventions.
