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

> Note: Use the `vppctl` binary from the same VPP build as the running VPP (e.g. `build-root/install-vpp-native/vpp/bin/vppctl`), not the distro one — version mismatch segfaults on the binary API.

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

---

## Load Generation (How to Run the Tests)

> **Important:** Run all commands from the **repository root** directory so that relative paths (`./bench/...` and `./flooder/...`) resolve correctly.

All measurements are captured through `bench/scripts/snapshot_metrics.sh`. This script clears VPP counters, runs a single load-generator command, and saves a consistent snapshot into `bench/results/snapshot/<label>/`.

First, export the exact path to your compiled VPP binary:
```bash
export VPPCTL_BIN="/path/to/vpp/build-root/install-vpp-native/vpp/bin/vppctl"
```

Build the custom flooder (used by several tests below):
```bash
# Run if you are at the repository root level.
# Otherwise, just navigate directly to the flooder directory.
cd flooder && cargo build --release
```

**Requirements before any run:**

1. VPP is running with the bench config.
2. A sink is listening in `ns-right` (open a dedicated terminal and keep it running):
   ```bash
   sudo ip netns exec ns-right iperf3 -s
   ```
3. **Flush the ARP caches after every VPP restart.** VPP generates a random MAC for its `af_packet` host interfaces at every start. Stale ARP entries in the namespaces will cause the kernel to send frames to the old MAC. VPP's `ethernet-input` will drop them (`l3 mac mismatch`), silently poisoning the test metrics.
   ```bash
   sudo ip netns exec ns-left ip neigh flush all
   sudo ip netns exec ns-right ip neigh flush all
   ```

### 1. Load Execution (Scaling Tests)

We measure VPP scaling behavior across custom paced rate limits and an unbounded saturation flood. We use 1 thread (`-T 1`) for paced traffic to ensure smooth packet delivery, and multiple threads (e.g., `-T 4`) for unbounded traffic to push the kernel to its absolute limits.

**Example 1: Paced Load (Replace `<rate>` and `<label>` as needed)**
```bash
# iperf3 baseline
sudo env VPPCTL_BIN="$VPPCTL_BIN" ./bench/scripts/snapshot_metrics.sh <label>_iperf -- \
  ip netns exec ns-left iperf3 -u -c 10.10.2.2 -b <rate> -l 1448 -t 30 -J

# flooder cross-check
sudo env VPPCTL_BIN="$VPPCTL_BIN" ./bench/scripts/snapshot_metrics.sh <label>_flooder -- \
  ip netns exec ns-left ./flooder/target/release/flooder -t 10.10.2.2:5201 -T 1 --rate <rate> -s 1448 -b 64 -d 30
```

**Example 2: Saturation Flood (Unbounded Ceiling)**
```bash
sudo env VPPCTL_BIN="$VPPCTL_BIN" ./bench/scripts/snapshot_metrics.sh ceiling_flooder -- \
  ip netns exec ns-left ./flooder/target/release/flooder -t 10.10.2.2:5201 -T 4 --rate 0 -s 1448 -b 64 -d 30
```

### 2. Passthrough A/B (FFI Cost Isolation)

To measure the raw overhead of the Rust classification call across the FFI boundary, toggle the node into pure forwarding mode and repeat your highest paced run and the ceiling run. 
**Both runs must happen in the same VPP session** to avoid CPU frequency drift.

```bash
# 1. Bypass Rust logic
sudo "$VPPCTL_BIN" rust-classify passthrough on

# 2. Run the highest paced payload (e.g., 1G)
sudo env VPPCTL_BIN="$VPPCTL_BIN" ./bench/scripts/snapshot_metrics.sh <label>_passthrough -- \
  ip netns exec ns-left ./flooder/target/release/flooder -t 10.10.2.2:5201 -T 1 --rate 1G -s 1448 -b 64 -d 30

# 3. Run the ceiling passthrough (Unbounded)
sudo env VPPCTL_BIN="$VPPCTL_BIN" ./bench/scripts/snapshot_metrics.sh ceiling_passthrough -- \
  ip netns exec ns-left ./flooder/target/release/flooder -t 10.10.2.2:5201 -T 4 --rate 0 -s 1448 -b 64 -d 30

# 4. Restore normal classification
sudo "$VPPCTL_BIN" rust-classify passthrough off
```
*Note: The FFI cost is calculated by subtracting the `Packet-Clocks` of the passthrough run from the normal run.*

_Validity check:_ `ip4-lookup` / `ip4-rewrite` / `ethernet-input` Packet-Clocks must be within a few percent between the two runs. 
Also note the passthrough run counts ALL frames in valid `udp packets` forwarded (including ARP), while the normal run counts only valid UDP — expect the passthrough counter to be a few packets higher.

### 3. Inspecting Results

Run these verification checks to validate data integrity before proceeding to analysis.

**Check 1: Clean Forwarding (No Drops on Paced Runs)**
Verify that all paced runs processed traffic perfectly without kernel or VPP-side drops.
*(Note: Replace `<paced_label_pattern>` with your actual directory pattern, intentionally excluding the `ceiling` runs which are expected to have drops).*

(If your pattern matches `passthrough` runs, note that the VPP counter is expected to be a few packets higher, because passthrough counts ARP frames too. Apply the "perfect match" rule only to non-passthrough directories.)

```bash
# For iperf3 runs (Expected: "lost_percent": 0)
grep -H "lost_percent" bench/results/snapshot/<paced_label_pattern>/loadgen_output.txt

# For flooder runs (Expected: Total Packets perfectly matches VPP's valid udp packets)
grep -H "Total Packets" bench/results/snapshot/<paced_label_pattern>/loadgen_output.txt
grep -H "valid udp packets" bench/results/snapshot/<paced_label_pattern>/show_errors.txt
```

**Check 2: ARP Contamination**
```bash
grep -H "mac mismatch" bench/results/snapshot/*/show_errors.txt
```
*Expected:* Should return no results (or exactly `0`). If you see `l3 mac mismatch`, the ARP cache was not flushed prior to the run, which silently poisons the cross-check by dropping packets at `ethernet-input` while `rust-classify` still counts them, heavily skewing `Packet-Clocks`.

**Check 3: Bottleneck Evidence (Ceiling Run)**
```bash
grep -H "Failed syscalls" bench/results/snapshot/*ceiling*/loadgen_output.txt
```
*Expected:* Massive loss in the unbounded run. Note that `Failed syscalls` (saturated sender socket / ENOBUFS) is a _minor_ contributor here — the bulk of the loss happens further along the pre-VPP path (`af_packet` RX ring on the VPP side), which is why VPP's counters fall far behind the flooder's `Total Packets` while VPP itself still drops nothing. This proves the bottleneck is the kernel-side networking stack, not VPP.

### 4. Profiling (FlameGraph)

While the bench is saturated, capture a CPU profile of the running VPP and render it as a flame graph (used as bottleneck evidence in `REPORT.md`):

> **Note:** This guide uses the FlameGraph scripts instead of Hotspot
> because Hotspot's GUI is unavailable on my WSL2.

```bash
# one-time: flame graph rendering scripts
git clone https://github.com/brendangregg/FlameGraph /tmp/FlameGraph

# terminal A: record for 45 s — it just waits, do NOT wait for it to finish
sudo perf record -F 99 -g -p "$(pidof vpp)" -- sleep 45

# terminal B, immediately (while A is waiting): unbounded ceiling run
sudo ip netns exec ns-left ./flooder/target/release/flooder \
  -t 10.10.2.2:5201 -T 4 --rate 0 -s 1448 -b 64 -d 30

# after both finish (perf.data is in the directory where perf record ran):
sudo perf report --stdio 2>/dev/null | sudo tee bench/perf_report_w2_ceiling.txt > /dev/null
sudo perf script | /tmp/FlameGraph/stackcollapse-perf.pl \
  | /tmp/FlameGraph/flamegraph.pl > bench/vpp_ceiling_flamegraph.svg
```

(`perf.data` is a raw binary profile — keep it out of the repository; only the rendered report and SVG are committed.)

Open the SVG in a browser: the dominant stack is `dispatch_pending_node` → the `af_packet` TX path (`sendto` → kernel softirq on the peer veth) — the kernel-side cost that sets the bench ceiling.
