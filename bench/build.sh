#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
vpp=${VPP_DIR:-/home/user/vpp}
profile=${1:-release}
[[ $profile == release || $profile == debug ]] || { echo 'Expected release or debug' >&2; exit 2; }
opt=-O3
build="$vpp/build-root/build-vpp-native/vpp"
if [[ $profile == debug ]]; then
  build="$vpp/build-root/build-vpp_debug-native/vpp"
  opt=-O0
fi
mkdir -p "bench/build/$profile"
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
RUSTFLAGS='-C relocation-model=pic' cargo build --release --locked --offline
# Match the local VPP ABI and baseline CPU target. Do not replace week3 links.
for source in plugin node; do
  clang -DHAVE_FCNTL64 -D_FORTIFY_SOURCE=2 -I"$vpp/src" \
    -I"$build/CMakeFiles" -I"$vpp/src/plugins" -Iinclude \
    -fPIC -g -Werror -Wall -Wno-address-of-packed-member "$opt" \
    -fstack-protector -fno-common -march=corei7 -mtune=corei7-avx \
    -fvisibility=hidden -ffunction-sections -fdata-sections \
    -c "plugin/$source.c" -o "bench/build/$profile/$source.o"
done
clang -shared -Wl,--gc-sections -o "bench/build/$profile/rust_classify_plugin.so" \
  "bench/build/$profile/plugin.o" "bench/build/$profile/node.o" \
  target/release/libnetwork_parser.a -lpthread -ldl -lm -lrt -lutil
