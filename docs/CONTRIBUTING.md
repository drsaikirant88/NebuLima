# Contributing to NebuLima

Thanks for considering a contribution. NebuLima is MIT-licensed, so you're free to
fork, modify, and redistribute it without restriction - but if you build something on
top of it or find it useful, a credit back to this repo (or better yet, a pull
request) is always appreciated.

## Before you start

Read [development docs](DEVELOPMENT.md) for the project layout, IPC
architecture, and the core principles this project follows, in particular:

- No telemetry/analytics/crash-reporting dependencies, ever.
- No new subprocess beyond `colima`, and no widening of the Tauri capability
  allowlist or shell-plugin scope, without flagging it explicitly in your PR
  description (see [security](SECURITY.md)).
- All UI colors must come from the locked six-color palette in [development docs](DEVELOPMENT.md#design-system).
- Every change needs tests; `make test-docker` must pass before a PR is opened.

## Setup

```sh
npm install
make dev          # run the app with hot-reload
```

See [build docs](docs/BUILD.md) for the full `make` target reference.

## Making a change

1. Create a branch off `main`.
2. Write tests alongside your change (TDD is expected, not optional).
3. Run `make lint` and `make test-docker` locally. `make test-docker` is the
   authoritative gate - it's what CI runs, and it's fully isolated from your host
   machine's Docker/Colima state.
4. Open a pull request describing *why* the change is needed, not just what changed.
   If it touches Tauri capabilities, shell-plugin scope, or Docker socket handling,
   say so explicitly and explain the security impact.

## Reporting bugs

- **Bugs/feature requests**: open a GitHub issue.
- **Security vulnerabilities**: open a Github issue and flag it.
