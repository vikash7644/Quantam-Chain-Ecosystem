#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR/.."

printf '%s\n' '[1/3] Checking Rust formatting'
cargo fmt --all -- --check

printf '%s\n' '[2/3] Running all targets and tests'
cargo test --all-targets

printf '%s\n' '[3/3] Building optimized release binaries'
cargo build --release

printf '%s\n' 'Fleur clean-environment verification passed.'
