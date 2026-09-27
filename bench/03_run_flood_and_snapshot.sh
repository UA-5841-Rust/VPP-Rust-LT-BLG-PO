#!/bin/bash

DELAY=$1
if [ -z "$DELAY" ]; then
  echo "Usage: $0 <delay_sec> (e.g., 0.01 for light, 0.001 for medium, 0 for max)"
  exit 1
fi

DURATION=10

echo "Clearing VPP counters..."
vppctl clear run
vppctl clear errors
vppctl clear hardware-interfaces

echo "Running custom UDP flood for ${DURATION} seconds (Delay: ${DELAY}s)..."
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 $DURATION $DELAY

echo "======================================"
echo "Metrics Snapshot (Load Level: Delay ${DELAY}s)"
echo "======================================"
echo "--- SHOW RUN ---"
vppctl show run