# Contributing to NebuLima

Thanks for considering a contribution. NebuLima is MIT-licensed, so you're free to
fork, modify, and redistribute it without restriction - but if you build something on
top of it or find it useful, a credit back to this repo (or better yet, a pull
request) is always appreciated.

## Before you start

Read [development docs](DEVELOPMENT.md) for the project layout, IPC
architecture, and the core principles this project follows, in particular:

- No telemetry/analytics/crash-reporting dependencies, ever.
- No widening of the Tauri capability allowlist or shell-plugin scope
  (`shell:allow-execute` is `colima` + `security`, nothing else reachable from the
  webview) without flagging it explicitly in your PR description. The same goes for
  any new native, non-webview-reachable subprocess call (like
  `preflight::fix_path_env`'s login-shell spawn) — see [security](SECURITY.md) for
  how that one is scoped and why it's a narrow exception.
- All UI colors must come from the locked six-color palette in [development docs](DEVELOPMENT.md#design-system).
- Every change needs tests; `make test` must pass before a PR is opened.

## Setup

```sh
make setup
make dev          # run the app with hot-reload
```

See [build docs](docs/BUILD.md) for the full `make` target reference.

## Making a change

1. Create a branch off `main`.
2. Write tests alongside your change (TDD is expected, not optional).
3. Run `make lint` and `make test` locally - that's what CI runs.
4. Open a pull request describing *why* the change is needed, not just what changed.
   If it touches Tauri capabilities, shell-plugin scope, or Docker socket handling,
   say so explicitly and explain the security impact.

A PR doesn't need to bump the version, but if it touches any of `package.json`,
`src-tauri/Cargo.toml`, or `src-tauri/tauri.conf.json`'s `version` field, CI's
version-check job will fail unless all three agree - see [releasing a new
version](#releasing-a-new-version).

## Releasing a new version

Releases are tag-triggered (`.github/workflows/release.yml`, on `v*.*.*` tags), not
automatic on merge. To cut one:

1. Bump the `version` field in all three of `package.json`, `src-tauri/Cargo.toml`,
   and `src-tauri/tauri.conf.json` to the same value - they drive the release bundle
   filenames, independent of whatever tag you push.
2. Merge that change to `main`.
3. `git tag -a vX.Y.Z -m "NebuLima vX.Y.Z"` then `git push origin vX.Y.Z`.
4. This builds and attaches `.deb`/`.AppImage`/`.dmg` artifacts as a **draft** GitHub
   release - review and publish it manually from the Releases tab.

## Reporting bugs

- **Bugs/feature requests**: open a GitHub issue.
- **Security vulnerabilities**: open a Github issue and flag it.
