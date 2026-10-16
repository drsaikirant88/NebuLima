# Development Guide

NebuLima is a Tauri v2 desktop app: a Rust backend (`src-tauri/`) that wraps the
`colima` CLI and talks to Docker over Colima's per-profile Unix socket, and a React +
Material UI frontend (`src/`) that drives it entirely through Tauri's IPC
command/event system. There is no web server mode and no browser build target.

## Core principles

1. **Native-only desktop architecture** — Tauri v2 + Rust backend + React/MUI frontend,
   no web/browser deployment target.
2. **Strict security & scope isolation** — no telemetry, Tauri capabilities
   whitelisted to exactly what the app needs, `tauri-plugin-shell` scoped to specific
   binaries/schemes only, Docker socket access restricted to `~/.colima/*/docker.sock`.
3. **Modular architecture & safe command abstraction** — UI components contain no
   business logic or CLI invocation; all shell/CLI execution goes through a dedicated,
   input-validating Rust abstraction.
4. **Test-first development with Docker-isolated testing** — tests accompany every
   change; `make test-docker` is the reproducible, host-independent gate.
5. **Strict design system** - Material + glassmorphism, locked six-color palette, both
   Dark and Light modes required.

Any change touching Tauri capabilities, shell-plugin scope, or socket-path validation
needs explicit security review!!

## Project layout

```text
src/                    React frontend
├── components/         UI, grouped by feature (profiles, containers, images, volumes, shell)
├── hooks/              Data-fetching/subscription hooks (e.g. useProfiles)
├── services/ipc.ts     The ONLY module that calls invoke()/listen() — see below
├── theme/              MUI theme, palette, glassmorphism helpers
└── types/ipc.ts        TypeScript mirrors of the Rust command payload/response shapes

src-tauri/src/          Rust backend
├── commands/           #[tauri::command] handlers, one module per feature area
├── colima/             colima CLI wrapper (cli.rs) and profile discovery/reconciliation (discovery.rs)
├── docker/             bollard-based Docker Engine client (containers, images, volumes)
├── diagnostics/        rotating log file setup (tracing-appender)
├── preflight.rs        colima/docker/kubectl availability checks
├── state.rs            shared app state + persisted settings
└── lib.rs              Tauri app bootstrap: plugin registration, command handler list, setup() hook

docs/                   This directory
tests/
├── backend/            Rust contract/unit tests (run by `cargo test`)
├── docker/             Docker Compose stack + Dockerfile for the authoritative test gate
└── frontend/           Vitest + React Testing Library specs
```

## IPC architecture

Every call from the frontend to the backend goes through `src/services/ipc.ts` — no
component calls `invoke()` or `listen()` directly. Each Tauri command in
`src-tauri/src/commands/*.rs` is registered in `lib.rs`'s `tauri::generate_handler![...]`
list, and its payload/response shape is mirrored in `src/types/ipc.ts`. When
adding a new command:

1. Write the Rust handler in the relevant `commands/*.rs` module, returning
   `Result<T, IpcError>`.
2. Register it in `lib.rs`'s `generate_handler!` list.
3. Add a typed wrapper in `src/services/ipc.ts`.
4. Add/update the matching type in `src/types/ipc.ts`.

## Testing

- `make test` — fast local loop: `cargo test` + `vitest run`.
- `make test-docker` — Runs the complete backend suite plus the frontend suite,
   fully isolated in a disposable Docker Compose stack (`tests/docker/docker-compose.yml`). 
   This is the one gate that must be green before any change is considered done — host-machine
   `cargo test`/`vitest` runs are a convenience, not a substitute.

See [build docs](BUILD.md) for the full command reference.

## Design system

All UI is Material UI, customized with a frosted-glass look
(`backdrop-filter: blur()` + semi-transparent fills), and every color anywhere in the
app — dark or light mode — must come from the six-color palette defined in
`src/theme/palette.ts`. No other hex values are permitted.

| Color       | Hex       | Preview |
|-------------|-----------|---------|
| white       | #FFFFFF   | <div style="width:24px;height:24px;background:#FFFFFF;border:1px solid #ccc;"></div> |
| lightGrey   | #EBEBEB   | <div style="width:24px;height:24px;background:#EBEBEB;border:1px solid #ccc;"></div> |
| deepInk     | #181722   | <div style="width:24px;height:24px;background:#181722;border:1px solid #ccc;"></div> |
| slate       | #393F4A   | <div style="width:24px;height:24px;background:#393F4A;border:1px solid #ccc;"></div> |
| warmSand    | #DCD2CC   | <div style="width:24px;height:24px;background:#DCD2CC;border:1px solid #ccc;"></div> |
| mutedMauve  | #8A7875   | <div style="width:24px;height:24px;background:#8A7875;border:1px solid #ccc;"></div> |
