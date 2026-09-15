# Local Bench Setup

This guide explains how to configure your environment and use the provided configuration files to run the local test bench.

## Environment Initialization

1.  **Stand up the topology:**
    ```bash
    # Note: If you encounter errors, clean up using ./teardown_topology.sh first, then retry.
    sudo ./setup_topology.sh
    ```

2.  **Copy the VPP configuration:**
    ```bash
    sudo cp ./vpp-bench.conf /path/to/vpp/
    sudo cp ./vpp-bench.cli /path/to/vpp/
    ```
    > **Note:** Ensure that the `exec` path in the `vpp-bench.conf` file accurately points to your local `vpp-bench.cli` file.

3.  **Build VPP:**
    ```bash
    # Navigate to the VPP source directory
    # Specify the absolute path to the network_parser library
    make build-release VPP_EXTRA_CMAKE_ARGS="-DNETWORK_PARSER_DIR=/path/to/network_parser"
    ```

4.  **Run VPP:**
    ```bash
    sudo build-root/install-vpp-native/vpp/bin/vpp -c vpp-bench.conf 
    ```

### Topology Overview

The local test bench topology consists of:
*   Two isolated network namespaces (`ns-left` and `ns-right`) connected through VPP running in the root namespace.
*   Two `veth` pairs. Each pair has one endpoint inside a namespace and the other in the root namespace, bound to VPP via `af_packet` host interfaces.

**Topology Diagram:**
```plaintext
   ns-left --veth-left----vpp-left--[ VPP ]--vpp-right----veth-right-- ns-right
           10.10.1.2/24  10.10.1.1/24       10.10.2.1/24  10.10.2.2/24
```

*   **Note:** The `vpp-left` and `vpp-right` host interfaces do not have kernel IP addresses assigned. VPP is the sole L3 forwarding participant on those endpoints.

---

## Verification and Baseline

Once VPP is running, verify the bench before executing any load tests.

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
    With the interface up but completely idle, the worker thread polls the interfaces (~15.4M loops/sec), but 0 vectors are processed. The `rust-classify` node shows exactly 0 calls and 0 vectors, meaning 0 cycles are spent processing.
    ```text
    Thread 1 vpp_wk_0 (lcore 2)
    Time 28.6, 10.000000 sec internal node vector rate 0.00 loops/sec 15442789.69
      vector rates in 0.0000e0, out 0.0000e0, drop 0.0000e0, punt 0.0000e0
                 Name                 State         Calls          Vectors        Suspends      Packet-Clocks   Vectors/Call  
    ```

*   **`show errors` (all at zero):** 
    All application-level traffic counters for our custom node remain strictly at **zero**. No packets were unexpectedly forwarded or dropped.
    ```text
       Count                  Node                              Reason               Severity 
    ```

*   **`show hardware-interfaces` (counters):** 
    The hardware interfaces reflect a completely idle network. RX and TX packet counts remain static, and the `af_packet` ring buffers show full availability with no pending blocks or drops (e.g., 5120 frames ready in the RX queue).
    ```text
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

We perform a sanity check on the bench with a small, manual amount of traffic first to confirm that packets are actually reaching and passing through the node. **Do not proceed to load generation until this is confirmed.**

```bash
# In namespace `ns-right` (sink):
sudo ip netns exec ns-right iperf3 -s

# In namespace `ns-left` (generator) - sending a handful of packets:
sudo ip netns exec ns-left iperf3 -u -c 10.10.2.2 -b 100k -t 2 -l 1200
```

Check the VPP trace to validate FFI parsing:
```plaintext
vpp# clear trace
vpp# trace add af-packet-input 10
# [Run iperf3 again]
vpp# show trace
```

**Trace Result (Valid UDP Packet):**
```text
00:01:00:784649: af-packet-input
  af_packet: hw_if_index 2 rx-queue 0 next-index 11
00:01:00:788085: rust-classify
  RUST-CLASSIFY: sw_if_index 2, next index 1, valid 1, protocol 1, dest_port 53355, error_code 0
00:01:00:788090: ethernet-input
  IP4: 12:d3:41:68:9b:14 -> 02:fe:05:ff:df:04
```

This explicit trace output (`valid 1`, `error_code 0`) confirms that real UDP traffic is successfully reaching the `rust-classify` node, crossing the zero-copy FFI boundary without errors, and being correctly parsed.
