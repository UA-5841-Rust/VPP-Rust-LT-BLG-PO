# Performance Report: VPP + Rust Node

## 1. Bench Topology & Setup (Part A)
* **Topology:** Variant 1 (veth-based, single host). Selected for consistent reproducibility on a local machine without requiring physical NICs. Two namespaces (`ns-gen` and `ns-sink`) are bridged by VPP via `af_packet`.
* **Zero-Load Baseline:** `Vectors/Call`: 1.00 | `Packet-Clocks`: ~2.65e4 (Interface up, completely idle).
* **Build:** VPP and Rust plugins are compiled in **Debug mode**, preserving `-O0` and assertion overheads to study unoptimized FFI boundary costs.

## 2. Load Generation & FFI Isolation (Part B)
**Generators Used:** 
1. `iperf3` (-u): Initial testing revealed `iperf3` mandates a TCP handshake (port 5201) before sending UDP. The Rust node correctly classified these as `UnsupportedProtocol` and dropped them, preventing load generation.
2. `Custom Python UDP Flooder`: Developed a dual-socket, lock-free script bypassing handshakes to simulate real UDP traffic bursts.

**Load Levels (Custom Flooder, Classifying Mode):**
| Load Level | Offered Load | Vectors/Call | Packet-Clocks | Mode |
| :--- | :--- | :--- | :--- | :--- |
| **Light** (`sleep 0.01`) | ~200 pps | 1.00 | ~1.75e4 | Interrupt |
| **Moderate** (`sleep 0.001`) | ~2000 pps | 7.27 | ~3.28e3 | Interrupt/Polling |
| **Saturated** (No sleep) | Max local CPU | ~163.50 | ~1.03e3 | Polling |

**Passthrough vs. Classifying (at Saturated Load):**
To isolate FFI cost, a build-time flag (`PASSTHROUGH_MODE`) bypassed `packet_classify`.
* **Classifying (Rust FFI):** Vectors: ~163.50 | Clocks: ~1.03e3
* **Passthrough (C Only):** Vectors: ~162.82 | Clocks: ~2.54e2
* **FFI Overhead:** ~776 CPU cycles per packet (amortized over large vectors).

## 3. Queues, Placement, and NUMA (Part C)
* **NUMA:** The system operates on a single NUMA node. Cross-socket penalties are not applicable.
* **Worker Configuration:** Reconfigured VPP with `workers 2` and ingress `num-rx-queues 2`. 
* **Placement Inspection:** Using `set interface rx-placement`, explicitly pinned `queue 0` to `worker 0` and `queue 1` to `worker 1`.

## 4. Bottleneck Analysis & Attempted Fix (Part D)
* **Hypothesis:** The primary bottleneck under light/moderate load is the fixed cost of the FFI context switch in Debug mode. Because VPP operates in `interrupt` mode (Vectors = 1.00), this heavy penalty (~17.5k clocks) cannot be amortized, keeping efficiency low.
* **Attempted Mitigation (Multi-worker Scaling):** Attempted to scale throughput by assigning 2 workers and 2 RX queues to handle distinct UDP flows from the custom generator. 
* **Outcome (Negative Result):** Throughput did not scale linearly, and load distribution was highly erratic (e.g., Thread 1 captured batches of ~7 while Thread 2 processed singles). *Reason:* Linux `veth` interfaces do not support hardware Receive Side Scaling (RSS). Traffic hashing relies on kernel software flow dissection, which fails to balance uniformly across `af_packet` rings compared to a physical NIC.
* **Proposed True Fix:** Compiling the environment in Release mode (`make build-release`) is the only definitive way to lower the base FFI parsing cost (~776 amortized cycles).

### Known Limitations
* **Load Generator Limits:** The Python generator shares the same CPU cores as VPP, creating artificial context-switching contention.
* **Hardware RSS:** Virtual `veth` interfaces lack RSS, meaning the multi-worker placement test could not demonstrate true parallel scaling.
* **CPU Isolation & Profiling:** Full CPU isolation via `isolcpus` and generating a Flame Graph via `perf record` were blocked by the WSL2 platform constraints (Microsoft custom kernel `6.6.87.2-microsoft` lacks standard `linux-perf` matching toolchains).