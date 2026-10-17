#!/usr/bin/env bash
# Test harness for scripts/check-platform.sh.
# Drives host OS via CHECK_PLATFORM_HOST_OS_OVERRIDE rather than stubbing `uname` on
# PATH, since execing a freshly-written stub binary isn't reliable in every sandbox.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK="$SCRIPT_DIR/check-platform.sh"
BASH_BIN="$(command -v bash)"
FAILURES=0

run_case() {
    local description="$1" host_os="$2" target="$3" expect_exit="$4" expect_message="$5"
    local output exit_code
    set +e
    output="$(CHECK_PLATFORM_HOST_OS_OVERRIDE="$host_os" "$BASH_BIN" "$CHECK" "$target" 2>&1)"
    exit_code=$?
    set -e

    if [ "$exit_code" -ne "$expect_exit" ]; then
        echo "FAIL: $description — expected exit $expect_exit, got $exit_code"
        echo "  output: $output"
        FAILURES=$((FAILURES + 1))
        return
    fi

    if ! grep -q "$expect_message" <<<"$output"; then
        echo "FAIL: $description — expected output to mention '$expect_message'"
        echo "  output: $output"
        FAILURES=$((FAILURES + 1))
        return
    fi

    echo "PASS: $description"
}

# Case 1: macos target on Darwin host → passes.
run_case "macos target on Darwin host" "Darwin" "macos" 0 "passed"

# Case 2: macos target on Linux host → fails naming build-macos.
run_case "macos target on Linux host" "Linux" "macos" 1 "build-macos"

# Case 3: linux target on Linux host → passes.
run_case "linux target on Linux host" "Linux" "linux" 0 "passed"

# Case 4: linux target on Darwin host → fails naming build-ubuntu.
run_case "linux target on Darwin host" "Darwin" "linux" 1 "build-ubuntu"

# Case 5: unknown target → fails.
run_case "unknown target" "Darwin" "windows" 1 "unknown platform"

if [ "$FAILURES" -ne 0 ]; then
    echo "$FAILURES test case(s) failed."
    exit 1
fi

echo "All check-platform.sh test cases passed."
