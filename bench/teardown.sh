#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
# Touch only bench-owned processes and named namespace interfaces.
for name in server vpp; do
  if [[ -f "$RUN/$name.pid" ]]; then
    pid=$(cat "$RUN/$name.pid")
    if [[ $pid =~ ^[0-9]+$ && -r /proc/$pid/cmdline ]]; then
      command=$(tr '\0' ' ' < "/proc/$pid/cmdline")
      if [[ $command == *"$RUN"* || ( $name == server && $command == *"$BENCH"* ) ]]; then
        kill "$pid" 2>/dev/null || true
        for _ in {1..30}; do kill -0 "$pid" 2>/dev/null || break; sleep .1; done
      else
        echo "Refusing to kill unrelated PID $pid" >&2
        exit 1
      fi
    fi
    rm -f "$RUN/$name.pid"
  fi
done
for side in a b; do
  ip netns del "rc-$side" 2>/dev/null || true
  ip link del "rc-$side-vpp" 2>/dev/null || true
done
rm -f "$RUN/cli.sock" "$RUN/stats.sock"
