#!/usr/bin/env bash
# Test harness for scripts/preflight-check.sh.
# Exercises all four PATH combinations of colima/docker presence by building a
# throwaway PATH directory containing only the stub binaries each case needs,
# rather than touching the real system PATH.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFLIGHT="$SCRIPT_DIR/preflight-check.sh"
# Captured before any test case restricts PATH, since a fully-restricted PATH would
# otherwise prevent the script's own "#!/usr/bin/env bash" shebang from resolving bash.
BASH_BIN="$(command -v bash)"
FAILURES=0

make_stub_path() {
    local dir="$1"
    shift
    mkdir -p "$dir"
    for name in "$@"; do
        cat >"$dir/$name" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
        chmod +x "$dir/$name"
    done
}

run_case() {
    local description="$1" stub_path="$2" expect_exit="$3" expect_message="$4"
    local output exit_code
    set +e
    output="$(PATH="$stub_path" "$BASH_BIN" "$PREFLIGHT" 2>&1)"
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

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# Case 1: colima missing, docker present → non-zero exit naming colima.
make_stub_path "$WORKDIR/docker-only" docker
run_case "colima missing" "$WORKDIR/docker-only" 1 "colima"

# Case 2: docker missing, colima present → non-zero exit naming docker.
make_stub_path "$WORKDIR/colima-only" colima
run_case "docker missing" "$WORKDIR/colima-only" 1 "docker"

# Case 3: both missing → non-zero exit naming both.
mkdir -p "$WORKDIR/empty"
run_case "both missing" "$WORKDIR/empty" 1 "colima"
run_case "both missing (docker named too)" "$WORKDIR/empty" 1 "docker"

# Case 4: both present → zero exit with a success message.
make_stub_path "$WORKDIR/both" colima docker
run_case "both present" "$WORKDIR/both" 0 "passed"

if [ "$FAILURES" -ne 0 ]; then
    echo "$FAILURES test case(s) failed."
    exit 1
fi

echo "All preflight-check.sh test cases passed."
