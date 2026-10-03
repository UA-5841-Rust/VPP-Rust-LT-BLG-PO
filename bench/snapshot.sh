#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
out=${1:?Usage: snapshot.sh output-directory}
mkdir -p "$out"
date --iso-8601=ns > "$out/time.txt"
for item in 'run' 'errors' 'hardware-interfaces' 'buffers' 'interface' 'interface rx-placement' 'threads'; do
  cli "show $item" > "$out/${item// /-}.txt"
done
"$VPP_BIN/vpp_get_stats" socket-name "$RUN/stats.sock" dump | gzip -n > "$out/stats.txt.gz"
ip -s -j link show > "$out/kernel-links.json"
for side in a b; do
  ip netns exec "rc-$side" ip -s -j link show > "$out/ns-$side.json"
  ip netns exec "rc-$side" cat /proc/net/snmp > "$out/ns-$side-snmp.txt"
  for flag in -S -g -l; do
    ethtool "$flag" "rc-$side-vpp" > "$out/ethtool-$side-${flag#-}.txt" 2>&1 || true
  done
done
cat /proc/net/softnet_stat > "$out/softnet.txt"
for status in /proc/"$(cat "$RUN/vpp.pid")"/task/*/status; do
  cat "$status"
done > "$out/thread-status.txt"
