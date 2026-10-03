#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked --offline
cc -std=c11 -Wall -Wextra -Werror -Iinclude tests/ffi_smoke.c \
  target/release/libnetwork_parser.a -lpthread -ldl -lm -lrt -lutil -o target/ffi-smoke
./target/ffi-smoke
