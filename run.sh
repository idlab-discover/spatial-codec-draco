#!/usr/bin/env bash
set -euo pipefail

# Small helper to run the example binary.
#
# Usage:
#   ./run.sh [--release] -- <example args>

RELEASE_FLAG=""
ARGS=()

for arg in "$@"; do
  if [[ "$arg" == "--release" ]]; then
    RELEASE_FLAG="--release"
  else
    ARGS+=("$arg")
  fi
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

exec cargo run ${RELEASE_FLAG} --example combine_folders -- "${ARGS[@]}"
