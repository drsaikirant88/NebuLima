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

NebuLima requires Node `>=26.13.0` (`.nvmrc` pins the floating major `26`). With
[nvm](https://github.com/nvm-sh/nvm) installed, run `nvm use` from the repo root before
`make setup` - `.npmrc`'s `engine-strict=true` makes `npm ci`/`npm install` refuse to
run on an older Node.

```sh
nvm use
make setup
make dev          # run the app with hot-reload
```

See [build docs](docs/BUILD.md) for the full `make` target reference.

## Making a change

1. Create a branch off `main`.
2. Write tests alongside your change (TDD is expected, not optional).
3. Run `make lint` and `make test` locally - CI runs the same checks, split across the
   `Frontend` and `Backend` jobs in `.github/workflows/ci.yml`.
4. Open a pull request titled as a [Conventional Commit](https://www.conventionalcommits.org/)
   (`type(scope): subject`, one of `feat|fix|perf|refactor|docs|test|build|ci|chore|revert`)
   describing _why_ the change is needed, not just what changed. PRs are squash-merged
   using the PR title as the commit message, so the title is what ends up in history.
   If it touches Tauri capabilities, shell-plugin scope, or Docker socket handling,
   say so explicitly and explain the security impact.

A PR doesn't need to bump the version - don't touch `package.json`, `src-tauri/Cargo.toml`,
or `src-tauri/tauri.conf.json`'s `version` field by hand. [release-please](#releasing-a-new-version)
owns those fields and opens its own PR for them.

## Releasing a new version

Releases are automated with [release-please](https://github.com/googleapis/release-please):
nobody bumps a version field or pushes a `v*` tag by hand.

1. Every PR merged to `main` must be titled as a Conventional Commit (enforced by the
   `PR title` check). `release-please.yml` runs on every push to `main` and keeps a
   standing **Release PR** up to date - it bumps `package.json`, `src-tauri/Cargo.toml`,
   and `src-tauri/tauri.conf.json`'s `version` fields together, updates `Cargo.lock`, and
   writes `CHANGELOG.md` from the merged commits since the last release. `fix:` bumps
   patch, `feat:` bumps minor, a `!` or `BREAKING CHANGE:` footer bumps major (minor while
   the project is still below `1.0.0`). `chore:`, `ci:`, `docs:`, `build:`, `test:`, and
   `refactor:` alone don't trigger a release.
2. **Merging the Release PR is the release button.** On merge, release-please tags
   `vX.Y.Z` and creates a **draft** GitHub release with the changelog as its body.
   `release.yml` then builds `.deb`/`.AppImage`/`.dmg` for every platform, a guard job
   checks the tag is actually on `main` and all three version fields agree, and a final
   job publishes the release once every build has attached its artifacts.
3. To force a specific version instead of letting commit types decide it, add a
   `Release-As: X.Y.Z` footer to a commit on `main` (most often the PR's squash commit
   message) - release-please picks this up on its next run.
4. macOS and Linux builds are unsigned. On recent macOS, this means Gatekeeper reports
   "NebuLima is damaged and can't be opened" rather than the older "unidentified
   developer" prompt, and right-click → Open no longer bypasses it. Clear the quarantine
   attribute instead: `xattr -cr /path/to/NebuLima.app`.

## Reporting bugs

- **Bugs/feature requests**: open a GitHub issue.
- **Security vulnerabilities**: see [reporting a vulnerability](SECURITY.md#reporting-a-vulnerability) - use
  GitHub's private vulnerability reporting instead of a public issue.
