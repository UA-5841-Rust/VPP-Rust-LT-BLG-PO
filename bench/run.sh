#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
tool=${1:?iperf or custom}
mode=${2:?classify or passthrough}
rate=${3:?rate in bits/sec (iperf) or packets/sec (custom)}
label=${4:?unique run label}
duration=${DURATION:-10}
[[ $tool == iperf || $tool == custom ]] || exit 2
[[ $mode == classify || $mode == passthrough ]] || exit 2
[[ $label =~ ^[a-zA-Z0-9_-]+$ && $rate =~ ^[0-9]+$ ]] || exit 2
out="$BENCH/results/$label"
[[ ! -e $out ]] || { echo "Run exists: $out" >&2; exit 1; }
mkdir -p "$out"
extra=()
[[ $mode == passthrough ]] && extra=(passthrough)
cli rust classify host-rc-a-vpp to host-rc-b-vpp "${extra[@]}"
cli clear errors
cli clear runtime
cli clear interfaces
bash "$BENCH/snapshot.sh" "$out/before"
printf '%s\n' "tool=$tool mode=$mode requested_rate=$rate duration=$duration socket_buffer=${SOCKET_BUFFER:-default}" > "$out/parameters.txt"
if [[ $tool == iperf ]]; then
  ip netns exec rc-b iperf3 -s -1 -J --logfile "$out/server.json" &
else
  ip netns exec rc-b python3 "$BENCH/udp_load.py" receive --output "$out/server.json" &
fi
server=$!
echo "$server" > "$RUN/server.pid"
cleanup() {
  kill "$server" 2>/dev/null || true
  wait "$server" 2>/dev/null || true
  rm -f "$RUN/server.pid"
}
trap cleanup EXIT
sleep .5
pid=$(cat "$RUN/vpp.pid")
"$PERF" stat -x ';' -e context-switches,cpu-migrations,task-clock,cycles,instructions \
  -p "$pid" -o "$out/perf-stat.txt" -- sleep "$duration" &
stat=$!
if [[ ${PROFILE:-0} == 1 ]]; then
  "$PERF" record -e cpu-clock -F 99 --call-graph dwarf -p "$pid" \
    -o "$out/perf.data" -- sleep "$duration" > "$out/perf-record.txt" 2>&1 &
  record=$!
fi
if [[ $tool == iperf ]]; then
  socket_args=()
  [[ -n ${SOCKET_BUFFER:-} ]] && socket_args=(-w "$SOCKET_BUFFER")
  ip netns exec rc-a taskset -c 6 iperf3 -c 10.44.0.2 -u -b "$rate" \
    -l 512 -t "$duration" "${socket_args[@]}" -J > "$out/client.json"
  wait "$server"
else
  ip netns exec rc-a taskset -c 6 python3 "$BENCH/udp_load.py" send \
    --pps "$rate" --seconds "$duration" --output "$out/client.json"
  sleep .5
  kill "$server"
  wait "$server"
fi
wait "$stat" || true
if [[ ${PROFILE:-0} == 1 ]]; then
  wait "$record" || true
  "$PERF" report -f --stdio -i "$out/perf.data" > "$out/perf-report.txt" 2>&1 || true
  "$PERF" script -f -i "$out/perf.data" > "$out/perf-script.txt" 2> "$out/perf-script-errors.txt" || true
  if [[ -s "$out/perf-script.txt" ]]; then
    python3 "$BENCH/flamegraph.py" "$out/perf-script.txt" "$out/flamegraph.svg"
  fi
fi
bash "$BENCH/snapshot.sh" "$out/after"
python3 - "$out" <<'PY'
import json, pathlib, sys
p = pathlib.Path(sys.argv[1])
c = json.loads((p / 'client.json').read_text())
if 'error' in c:
    raise SystemExit(c['error'])
if 'sent_packets' in c:
    s = json.loads((p / 'server.json').read_text())
    c['received_packets'] = s['received_packets']
    c['loss_percent'] = 100 * (c['sent_packets'] - s['received_packets']) / max(c['sent_packets'], 1)
else:
    sent = c['end']['sum_sent']
    recv = c['end']['sum_received']
    c = dict(offered_pps=sent['packets'] / sent['seconds'],
             offered_payload_bps=sent['bits_per_second'],
             sent_packets=sent['packets'], received_packets=recv['packets'] - recv['lost_packets'],
             loss_percent=recv['lost_percent'], jitter_ms=recv['jitter_ms'])
(p / 'summary.json').write_text(json.dumps(c, indent=2) + '\n')
PY
echo "Captured $label"
