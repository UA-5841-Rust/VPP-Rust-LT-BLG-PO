# Local bench setup

Here you can find the way you need to configure your environment and which configuration files use to run the bench.

## To initialize the environment:

1.  **Stand up the topology:**
    ```bash
    # Make sure to clean it up using `teardown_topology.sh` and run script below again if you get some error here
    sudo ./setup_topology.sh
    ```

2. **Copy VPP configuration:**
    ```bash
    sudo cp ./vpp-bench.conf /path/to/vpp/
    sudo cp ./vpp-bench.cli /path/to/vpp/
    ```
    > Please don't forget to change the path `exec /path/to/vpp-bench.cli` in `vpp-bench.conf` configuration file

3.  **Build VPP:**
    ```bash
    # Make sure you are into /path/to/vpp dir :-)
    # Specify path to `network_parser` lib
    make build-release VPP_EXTRA_CMAKE_ARGS="-DNETWORK_PARSER_DIR=/path/to/network_parser"
    ```

4.  **Run VPP:**
    ```bash
    sudo build-root/install-vpp-native/vpp/bin/vpp -c vpp-bench.conf 
    ```

Talking about our current bench topology we set up above:

* Two `netns` connected through VPP running in the root `netns`
* Each `veth` pair has one end in a namespace and one end left in the root netns for VPP's `af_packet` host-interfaces to bind to.

The way it looks like:

```plaintext
   ns-left --veth-left----vpp-left--[ VPP ]--vpp-right----veth-right-- ns-right
           10.10.1.2/24  10.10.1.1/24       10.10.2.1/24  10.10.2.2/24
```

* `vpp-left`/`vpp-right` get no kernel IP — VPP is the only L3 participant on those addresses.

---

## Verification and Baseline

Once VPP is running, verify the bench before executing load tests.

### 1. Zero-Load Baseline

To record the idle state of the custom `rust-classify` node, we capture the baseline metrics while the interface is up but there is no active load. **This baseline is what every later measurement will be compared against.**

```bash
sudo vppctl clear run
sudo vppctl clear errors
sudo vppctl clear hardware-interfaces
# [Wait 10 seconds]
sudo vppctl show run
sudo vppctl show errors
sudo vppctl show hardware-interfaces
```

**What "zero load" looks like:**

*   **`show run` (cycles/vector):** 
    With the interface up but completely idle, the worker thread polls the interface (~15.4M loops/sec), but 0 vectors are processed. The `rust-classify` node shows exactly 0 calls and 0 vectors, meaning 0 cycles are spent processing.
    ```plaintext
    Thread 1 vpp_wk_0 (lcore 2)
    Time 28.6, 10.000000 sec internal node vector rate 0.00 loops/sec 15442789.69
      vector rates in 0.0000e0, out 0.0000e0, drop 0.0000e0, punt 0.0000e0
                 Name                 State         Calls          Vectors        Suspends      Packet-Clocks   Vectors/Call  
    ```

*   **`show errors` (all at zero):** 
    All application-level traffic counters for our custom node remain strictly at **zero**. No packets were unexpectedly forwarded or dropped.
    ```plaintext
       Count                  Node                              Reason               Severity 
    ```

*   **`show hardware-interfaces` (counters):** 
    The hardware interfaces reflect a completely idle network. RX and TX packet counts remain static, and the `af_packet` ring buffers show full availability with no pending blocks or drops (e.g., 5120 frames ready in the RX queue).
    ```plaintext
                 Name                Idx   Link  Hardware
    host-vpp-left                      1     up   host-vpp-left
      RX Queues:
        queue thread         mode      
        0     vpp_wk_0 (1)   interrupt 
      TX Queues:
        TX Hash: [name: hash-eth-l34 priority: 50 description: Hash ethernet L34 headers]
        queue shared thread(s)      
        0     yes    0-1
      Linux PACKET socket interface v3
      RX Queue 0:
        block size:65536 nr:160  frame size:2048 nr:5120 next block:18
      TX Queue 0:
        block size:69206016 nr:1  frame size:67584 nr:1024 next frame:9
        available:1024 request:0 sending:0 wrong:0 total:1024
    ```

### 2. Functional Sanity Check

Sanity check the bench with a small, manual amount of traffic first to confirm that packets are actually reaching and passing through the node. **Do not proceed to load generation until this is confirmed.**

```bash
# In namespace `ns-right` (sink):
sudo ip netns exec ns-right iperf3 -s

# In namespace `ns-left` (generator) - sending a handful of packets:
sudo ip netns exec ns-left iperf3 -u -c 10.10.2.2 -b 100k -t 2 -l 1200
```

Check VPP trace to validate FFI parsing:
```plaintext
vpp# clear trace
vpp# trace add af-packet-input 10
# [Run iperf3 again]
vpp# show trace
```

**Trace Result (Valid UDP Packet):**
```plaintext
00:01:00:784649: af-packet-input
  af_packet: hw_if_index 2 rx-queue 0 next-index 11
00:01:00:788085: rust-classify
  RUST-CLASSIFY: sw_if_index 2, next index 1, valid 1, protocol 1, dest_port 53355, error_code 0
00:01:00:788090: ethernet-input
  IP4: 12:d3:41:68:9b:14 -> 02:fe:05:ff:df:04
```

This explicit trace output (`valid 1`, `error_code 0`) confirms that real UDP traffic is successfully reaching the `rust-classify` node, crossing the zero-copy FFI boundary, and being correctly parsed.
