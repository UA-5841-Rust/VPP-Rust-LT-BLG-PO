#!/bin/bash
set -e

echo "--- 1. ZERO-LOAD BASELINE ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sleep 2
echo "> show run" > zero_load.txt
vppctl show run >> zero_load.txt
echo "> show errors" >> zero_load.txt
vppctl show errors >> zero_load.txt
echo "> show hardware-interfaces" >> zero_load.txt
vppctl show hardware-interfaces >> zero_load.txt
cat zero_load.txt

echo -e "\n--- 2. STARTING SINK LISTENER ---"
sudo ip netns exec ns-sink nc -u -l -p 5678 > /dev/null &
SINK_PID=$!

echo -e "\n--- 3. LOAD TEST: Python UDP (Low Load - 0.01s delay) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 10 0.01
vppctl show run && vppctl show errors

echo -e "\n--- 4. LOAD TEST: Python UDP (Medium Load - 0.001s delay) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 10 0.001
vppctl show run && vppctl show errors

echo -e "\n--- 5. LOAD TEST: Python UDP (Max Load - 0s delay) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 10 0
vppctl show run && vppctl show errors

echo -e "\n--- 6. CROSS-CHECK: hping3 (Max Load) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load hping3 -2 -p 5678 -i u10 -c 50000 10.10.2.2
vppctl show run && vppctl show errors && vppctl show hardware-interfaces

sudo kill $SINK_PID 2>/dev/null