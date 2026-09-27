#!/bin/bash
echo "1. Starting load generator in background..."
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 30 0 &
FLOOD_PID=$!

# Explicitly use the newly compiled binary
PERF_BIN="/usr/local/bin/perf"

echo "2. Collecting VPP profile (10 seconds)..."
# Write intermediate data to /tmp to bypass WSL Windows mount filesystem issues
sudo $PERF_BIN record -F 99 -p $(pidof vpp) -g -o /tmp/perf.data -- sleep 10

echo "3. Generating SVG graph..."
sudo $PERF_BIN script -i /tmp/perf.data | ~/FlameGraph/stackcollapse-perf.pl | ~/FlameGraph/flamegraph.pl > vpp_ceiling_flamegraph.svg

echo "Stopping load generator..."
sudo kill $FLOOD_PID 2>/dev/null
echo "Success! Graph saved to vpp_ceiling_flamegraph.svg."
