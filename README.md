# NebuLima

[![CI](https://github.com/drsaikirant88/NebuLima/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/drsaikirant88/NebuLima/actions/workflows/ci.yml)
[![Release](https://github.com/drsaikirant88/NebuLima/actions/workflows/release.yml/badge.svg)](https://github.com/drsaikirant88/NebuLima/actions/workflows/release.yml)
![Rust](https://img.shields.io/badge/rust-1.99-orange)
![Node](https://img.shields.io/badge/node-%3E%3D26-brightgreen)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Native desktop application for Colima — manage Colima VM profiles, and the containers,
images, and volumes running inside them, from a single control center.

> **Disclaimer**: NebuLima is built and validated against Colima v0.10.3, Docker v29.4.3,
> and kubectl v1.37.1; other versions are not guaranteed to work. It is provided as-is,
> with no warranty of any kind — use at your own risk. The maintainers assume no
> responsibility for any issues, data loss, or damages arising from its use.

## Installation

NebuLima requires the `colima` and `docker` CLIs to already be installed and on `PATH`.
NebuLima runs a pre-flight check at first launch, before its main window appears, and
refuses to open if either tool is absent from `PATH`.

### macOS

1. Install the prerequisites (via [Homebrew](https://brew.sh)):
   ```sh
   brew install colima docker
   ```
2. Download the latest `.dmg` (or build it yourself — see
   [build](docs/BUILD.md)) and drag `NebuLima.app` into `/Applications` or
   `$HOME/Applications`.
3. Launch NebuLima from Launchpad or Spotlight.

### Ubuntu

1. Install Docker's CLI following
   [Docker's official Ubuntu install guide](https://docs.docker.com/engine/install/ubuntu/).
2. Install Colima — the simplest path is via [Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux):
   ```sh
   brew install colima
   ```
3. Download the latest `.deb` or `.AppImage` (or build it yourself) and install it:
   ```sh
   sudo apt install ./NebuLima_<version>_<arch>.deb
   # or, for the AppImage:
   chmod +x NebuLima_<version>_<arch>.AppImage
   ./NebuLima_<version>_<arch>.AppImage
   ```
   (`<arch>` is `amd64` for the `.deb`, `x86_64` for the `.AppImage` — the release
   pipeline currently only builds for x86_64 Linux.)

If `colima` or `docker` is missing from `PATH`, NebuLima's main window will refuse to
appear the first time you launch it after installing, and an error dialog will name
exactly which tool(s) are missing — install them and relaunch the app.

Having trouble? See [troubleshooting](docs/TROUBLESHOOTING.md).

## Documentation

| Doc | What's in it |
| --- | --- |
| [build](docs/BUILD.md) | Every `make` target: dev, test, lint, format, release packaging. |
| [troubleshooting](docs/TROUBLESHOOTING.md) | Common launch/profile/socket problems and how to resolve them. |
| [development](docs/DEVELOPMENT.md) | Project layout, IPC architecture, testing strategy, constitution principles. |
| [security](docs/SECURITY.md) | Threat model, Tauri capability allowlist rationale, how to report a vulnerability. |
| [contributing](docs/CONTRIBUTING.md) | How to set up, make a change, and open a pull request. |

## Contributing

Contributions are welcome — see [contributing](docs/CONTRIBUTING.md). NebuLima is
[MIT-licensed](LICENSE): you're free to use, modify, and redistribute it, though a
credit back to this repo (or a pull request) is always appreciated.
