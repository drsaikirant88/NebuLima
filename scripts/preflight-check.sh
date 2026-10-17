#!/usr/bin/env bash
# Pre-flight check script for NebuLima
# Verifies that both colima and docker CLI tools are installed and accessible on PATH.
set -euo pipefail

MISSING=()

if ! command -v colima >/dev/null 2>&1; then
    MISSING+=("colima")
fi

if ! command -v docker >/dev/null 2>&1; then
    MISSING+=("docker")
fi

if [ ${#MISSING[@]} -ne 0 ]; then
    echo "ERROR: Missing required dependencies on PATH: ${MISSING[*]}" >&2
    echo "Please install the missing tools before proceeding with NebuLima build or execution." >&2
    exit 1
fi

echo "Pre-flight check passed: colima and docker are available on PATH."
exit 0

