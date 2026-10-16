# Docker environment for NebuLima development and testing
# Rust toolchain pinned to 1.99.0
FROM rust:1.99.0-slim-trixie AS base

# Install system dependencies for Tauri v2 Linux build & Docker CLI
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    wget \
    file \
    git \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libssl-dev \
    libayatana-appindicator3-dev \
    pkg-config \
    docker.io \
    docker-cli \
    docker-buildx \
    ca-certificates \
    gnupg \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Trust an injected CA (e.g. a corporate/sandbox TLS-intercepting proxy) for the rest of
# this build. The secret is optional: an absent or empty mount leaves the system trust
# store untouched, so this is a no-op on networks that don't intercept TLS.
RUN --mount=type=secret,id=ca_cert,required=false,mode=0444 \
    sh -c 'if [ -s /run/secrets/ca_cert ]; then \
      cp /run/secrets/ca_cert /usr/local/share/ca-certificates/sandbox-ca.crt && \
      update-ca-certificates; \
    fi'

# Install Node.js 26 (latest release line, e.g. v26.11.1)
# Attempts NodeSource setup_26.x or falls back to official nodejs.org Linux x64 binary distribution
RUN --mount=type=secret,id=ca_cert,required=false,mode=0444 \
    sh -c 'if [ -s /run/secrets/ca_cert ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/ca_cert; fi && \
      (curl -fsSL https://deb.nodesource.com/setup_26.x | bash - && apt-get install -y --no-install-recommends nodejs) \
      || (curl -fsSL https://nodejs.org/dist/latest-v26.x/node-v26.11.1-linux-x64.tar.xz | tar -xJf - -C /usr/local --strip-components=1)' \
    && node --version \
    && npm --version \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Pre-cache directory for offline development
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup

# Install the pinned cargo-tauri CLI (2.12.2). Trust the same optional CA for crates.io.
RUN --mount=type=secret,id=ca_cert,required=false,mode=0444 \
    sh -c 'if [ -s /run/secrets/ca_cert ]; then export CARGO_HTTP_CAINFO=/run/secrets/ca_cert; fi && \
      cargo install tauri-cli --version "2.12.2" --locked' || true

# Warm the cargo registry cache from the real crate so `cargo test` at container
# runtime (against the bind-mounted /workspace) can resolve/build without needing the
# secret again - CARGO_HOME lives outside /workspace so this cache survives the mount.
# The full src-tauri tree is copied (not just Cargo.toml) because cargo validates
# every manifest target path (e.g. the `[lib]` entry at src/lib.rs) before fetching.
COPY src-tauri src-tauri
RUN --mount=type=secret,id=ca_cert,required=false,mode=0444 \
    sh -c 'if [ -s /run/secrets/ca_cert ]; then export CARGO_HTTP_CAINFO=/run/secrets/ca_cert; fi && \
      cd src-tauri && cargo fetch' || true

# Warm npm's dependency tree into an image-local node_modules. The compose file mounts
# a named volume at /workspace/node_modules so this baked-in install survives the
# runtime bind-mount of the rest of the repo over /workspace. Falls back to `npm install`
# when no lockfile is committed yet (npm ci requires one to already exist).
COPY package.json package-lock.json* ./
RUN --mount=type=secret,id=ca_cert,required=false,mode=0444 \
    sh -c 'if [ -s /run/secrets/ca_cert ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/ca_cert; fi && \
      if [ -f package-lock.json ]; then npm ci; else npm install; fi'

CMD ["/bin/bash"]
