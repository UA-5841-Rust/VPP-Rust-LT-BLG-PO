#!/usr/bin/env bash
set -euo pipefail
BENCH=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
VPP_DIR=${VPP_DIR:-/home/user/vpp}
VPP_BIN="$VPP_DIR/build-root/install-vpp-native/vpp/bin"
RUN=${RUN:-/tmp/rc-week4}
perf_candidates=(/usr/lib/linux-tools/*/perf)
PERF=${PERF:-${perf_candidates[-1]}}
cli() {
  local reply first_line
  reply=$("$VPP_BIN/vppctl" -s "$RUN/cli.sock" "$@") || return
  printf '%s\n' "$reply"
  # vppctl may exit 0 even when a CLI handler returned an error.
  first_line=${reply%%$'\n'*}
  if [[ $first_line =~ :\ (unknown|failed|parse\ error|please\ specify|feature\ reset|feature\ enable|specify\ an|subinterfaces) ]]; then
    return 1
  fi
}
need_root() { [[ $EUID == 0 ]] || { echo 'Run with sudo (or WSL -u root).' >&2; exit 1; }; }
