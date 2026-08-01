#!/usr/bin/env bash
# Build, test, and demo-run the diagnostic-coordinates crate.
#
# Usage:
#   ./test.sh              # run from the crate root (where Cargo.toml lives)
#   ./test.sh /path/to/crate

set -euo pipefail

CRATE_DIR="${1:-.}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust (https://rustup.rs) first." >&2
  exit 1
fi

if [ ! -f "$CRATE_DIR/Cargo.toml" ]; then
  echo "error: no Cargo.toml found in '$CRATE_DIR'." >&2
  echo "Pass the crate root as an argument, e.g. ./test.sh /path/to/diagnostic-coordinates" >&2
  exit 1
fi

cd "$CRATE_DIR"

echo "== building =="
cargo build

echo
echo "== running tests =="
cargo test

echo
echo "== running toy_scenario example =="
cargo run --example toy_scenario

echo
echo "all checks passed."
