#!/usr/bin/env bash
# Verifies the host OS matches the platform a packaging target is about to build for.
# Tauri cannot cross-compile platform bundles - running build-macos on Linux (or
# build-ubuntu on macOS) fails deep inside the bundler with a confusing CLI error
# instead of a clear message, so this check fails fast before any build work starts.
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <macos|linux>" >&2
    exit 1
fi

TARGET="$1"
# Override seam for check-platform.test.sh, which can't safely exec a freshly-written
# stub binary in every environment. Production invocations always read the real uname.
HOST_OS="${CHECK_PLATFORM_HOST_OS_OVERRIDE:-$(uname -s)}"

case "$TARGET" in
    macos)
        if [ "$HOST_OS" != "Darwin" ]; then
            echo "ERROR: build-macos must run on macOS (Darwin), but this host reports '$HOST_OS'." >&2
            echo "Tauri cannot cross-compile a macOS .app/.dmg bundle from another OS." >&2
            exit 1
        fi
        ;;
    linux)
        if [ "$HOST_OS" != "Linux" ]; then
            echo "ERROR: build-ubuntu must run on Linux, but this host reports '$HOST_OS'." >&2
            echo "Tauri cannot cross-compile a .deb/.AppImage bundle from another OS." >&2
            exit 1
        fi
        ;;
    *)
        echo "ERROR: unknown platform target '$TARGET' (expected 'macos' or 'linux')." >&2
        exit 1
        ;;
esac

echo "Platform check passed: host OS '$HOST_OS' matches target '$TARGET'."
exit 0
