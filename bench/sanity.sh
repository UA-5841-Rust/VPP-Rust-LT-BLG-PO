#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
out=${1:-$BENCH/results/sanity}
mkdir -p "$out"
server=''
cleanup_sanity() {
  local status=$?
  if [[ -n $server ]]; then
    kill "$server" 2>/dev/null || true
    wait "$server" 2>/dev/null || true
  fi
  cli show trace > "$out/trace.txt" || true
  cli show errors > "$out/errors.txt" || true
  cli show interface > "$out/interfaces.txt" || true
  return "$status"
}
trap cleanup_sanity EXIT
cli clear trace
cli trace add af-packet-input 100
ip netns exec rc-b iperf3 -s -1 -J > "$out/server.json" 2>&1 &
server=$!
sleep .5
ip netns exec rc-a iperf3 -c 10.44.0.2 -u -b 100K -l 512 -t 2 -J > "$out/client.json"
wait "$server"
server=''
cli show trace > "$out/trace.txt"
grep -q 'rust-classify: protocol 1 .* valid 1 error 0' "$out/trace.txt"
python3 - "$out/client.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
assert data['end']['sum_received']['packets'] > 0, 'Sink received no UDP'
PY
trap - EXIT
cli clear trace
echo 'External UDP reached Rust and the sink; trace verified.'
