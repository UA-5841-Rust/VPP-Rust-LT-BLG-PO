#!/bin/bash
set -e

if ! command -v hping3 &> /dev/null; then
    echo "Error: hping3 is not installed. Please run: sudo apt install hping3"
    exit 1
fi

echo "--- 1. ZERO-LOAD BASELINE ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sleep 2
{ echo "> show run"; vppctl show run; echo "> show errors"; vppctl show errors; echo "> show hardware-interfaces"; vppctl show hardware-interfaces; } > zero_load.txt
cat zero_load.txt

echo -e "\n--- 2. STARTING SINK LISTENER ---"
sudo ip netns exec ns-sink nc -u -l -p 5678 > /dev/null &
SINK_PID=$!

echo -e "\n--- 3. LOAD TEST: Python UDP (Low Load - 0.01s delay) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 10 0.01
{ echo "> show run"; vppctl show run; echo "> show errors"; vppctl show errors; } > results_low.txt
cat results_low.txt

echo -e "\n--- 4. LOAD TEST: Python UDP (Medium Load - 0.001s delay) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 10 0.001
{ echo "> show run"; vppctl show run; echo "> show errors"; vppctl show errors; } > results_medium.txt
cat results_medium.txt

echo -e "\n--- 5. LOAD TEST: Python UDP (Max Load - 0s delay) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load ./udp_flood.py 10.10.2.2 5678 10 0
{ echo "> show run"; vppctl show run; echo "> show errors"; vppctl show errors; } > results_max.txt
cat results_max.txt

echo -e "\n--- 6. CROSS-CHECK: hping3 (High Load) ---"
vppctl clear run && vppctl clear errors && vppctl clear hardware-interfaces
sudo ip netns exec ns-load hping3 -2 -p 5678 -i u0 -c 100000 10.10.2.2
{ echo "> show run"; vppctl show run; echo "> show errors"; vppctl show errors; echo "> show hardware-interfaces"; vppctl show hardware-interfaces; } > results_hping3.txt
cat results_hping3.txt

echo -e "\n--- 7. RX-PLACEMENT VERIFICATION ---"
vppctl show interface rx-placement > rx_placement.txt
cat rx_placement.txt

sudo kill $SINK_PID 2>/dev/null || true
echo "All tests finished. Results saved to .txt files."