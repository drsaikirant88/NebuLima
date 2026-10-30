# Security

NebuLima runs with direct access to your Docker/Colima environment, so its security
model is deliberately narrow. This document describes what the app does, what it
refuses to do, and how to report a problem.

## Threat model & design constraints

NebuLima is a native desktop app (Tauri v2 + Rust backend, React/MUI frontend) built
to do exactly one thing: manage Colima VM profiles and the containers/images/volumes
inside them. It is intentionally locked down:

- **No telemetry.** The app contains no analytics, crash-reporting, or external
  tracking library, bundled or networked. Nothing about your usage leaves your machine.
- **No general-purpose file, process, or network tooling.** The app does not expose a
  file browser, an arbitrary shell/terminal, or any network client beyond talking to
  Colima's own Docker socket.
- **Frontend has no unmediated OS access.** The React UI never touches the file
  system, network, or a subprocess directly — every privileged action is a specific,
  hand-written Rust `#[tauri::command]`, reached only through
  `src/services/ipc.ts` (see [development](DEVELOPMENT.md)).
- **Kubernetes introspection is a narrow, read-only exception.** The backend's
  `kubernetes::client` module (`src-tauri/src/kubernetes/client.rs`) may talk directly
  to a Colima profile's embedded Kubernetes API server - the one exception to the
  "no network tooling beyond the Docker socket" rule — but only to
  read node/pod status, never to apply, create, update, or delete a cluster resource.
  It connects exclusively to the API server endpoint named in the kubeconfig context
  that `colima` itself wrote to disk for that profile; it never accepts a user-supplied
  or remote endpoint. All cluster _lifecycle_ actions (start/stop/reset/delete) still
  go exclusively through the `colima` subprocess, same as every other Colima command -
  this exception does not widen subprocess sandboxing. Like every other privileged
  capability, this client is reachable from the frontend only through its own
  `#[tauri::command]` wrappers via `src/services/ipc.ts`, never directly.
- **Login-shell `PATH` resolution is a narrow, startup-only exception.** GUI-launched
  apps (Finder/Dock/Spotlight) are spawned by `launchd`, the session manager with a
  bare-minimum `PATH` that never sources `.zprofile`/`.bash_profile`/etc., making
  Homebrew or standalone `colima`/`docker` invisible even when correctly installed.
  `preflight::fix_path_env` (`src-tauri/src/preflight.rs`) works around this by running
  a fixed, argument-free command — `$SHELL -lc 'printf %s "$PATH"'` — once during
  startup, before the main window is created. This is a native `std::process::Command`
  call, entirely separate from the `shell:allow-execute` Tauri capability below: it is
  never reachable from the webview/frontend, takes no user or IPC-supplied input
  (only the user's own `$SHELL` env var selects which binary runs), and the command
  string itself never changes. It does not widen what the webview can execute.

## Tauri capability allowlist

`src-tauri/capabilities/default.json` is the single source of truth for what the
webview is allowed to invoke. Every entry is deliberate:

| Permission                                                                        | Why it's needed                                                                                                                                                                                                  |
| --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `core:default`                                                                    | Baseline read-only app/window/event APIs (e.g. app name/version for the About popover).                                                                                                                          |
| `core:window:allow-close` / `allow-minimize` / `allow-toggle-maximize`            | The custom titlebar's traffic-light buttons — the OS draws no native titlebar (`decorations: false`), so these must be explicit commands.                                                                        |
| `core:window:allow-start-dragging`                                                | Lets the custom titlebar be dragged to move the window, for the same reason as above.                                                                                                                            |
| `shell:allow-execute` scoped to `colima`                                          | The _only_ subprocess NebuLima can launch. The scope hardcodes the binary name to `colima`; arguments are validated and constructed by `src-tauri/src/colima/cli.rs`, never built from unsanitized user strings. |
| `shell:default` (`allow-open`, scoped to `http(s)://`, `tel:`, `mailto:` schemes) | Lets the About popover's GitHub link open in the user's default browser. This does not grant arbitrary process execution — it hands a URL to the OS's own "open" handler, restricted to those schemes.           |

Widening this file, or the shell-plugin scope within it, is treated as a
security-relevant change: it must be called out explicitly in the pull request
description.

## Docker socket hardening

The backend will only read or write a Unix socket matching
`~/.colima/<profile-name>/docker.sock` — never `/var/run/docker.sock` or any other
path. Any request for a socket path outside that pattern is rejected before any I/O is
attempted (`src-tauri/src/docker` + the socket-path validation tests in
`tests/backend/socket_validation_test.rs`).

## Content Security Policy

The webview's CSP (`src-tauri/tauri.conf.json`) is locked to
`default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset:;` —
no remote scripts, no remote stylesheets, no arbitrary remote images.

## Reporting a vulnerability

Please use [GitHub's private vulnerability reporting](https://github.com/drsaikirant88/NebuLima/security/advisories/new)
for this repository instead of opening a public issue, so the report isn't visible
before a fix is ready.
