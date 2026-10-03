# VPP + Rust: Local Test Bench, Load Generation, and Performance Observability

## Practical Assignment: Building a Reproducible Local Bench for the Rust-Classify Node

### Overview

Task_03 gave you a working VPP node that calls into your `network_parser` Rust library across an FFI boundary to classify packets. So far, correctness was verified with synthetic traffic (`packet-generator`) and small `make test` cases — enough to prove the logic works, but not enough to say anything about how the node behaves under real load.

This assignment is about closing that gap. You will:

* Build a **repeatable local test bench**: two network namespaces (or two VMs/containers if you prefer) connected through a Linux interface that VPP with your `rust_classify_plugin` sits in the middle of, so real generated traffic — not just `packet-generator` streams — actually flows through your node.
* Generate load against that bench using external tools (`iperf3`, `t-rex`/`trex`, or a custom Rust/Go/Scapy-based flooder), at several distinct load levels.
* Instrument the bench: VPP's stats segment, `show run`, `show errors`, `show hardware-interfaces`, NIC ring/queue counters, and `perf`, to answer concrete questions about where time goes and where packets are lost.
* Produce a short performance report correlating **offered load → node counters → CPU/queue behavior**, and use it to find and (attempt to) fix at least one real bottleneck.

This is deliberately less prescriptive than task_01–task_03: part of the exercise is deciding what to measure and how, then defending those choices during review.

---

## Learning Objectives

After completing this assignment, you should be able to:

* Set up a reproducible local environment where real traffic (not only `packet-generator` streams) is routed through a custom VPP node.
* Use external load-generation tools against a VPP-fronted target and reason about what "load" actually means in each case (PPS vs. bandwidth vs. connections/sec).
* Read and interpret `show run`, `show errors`, `show hardware-interfaces`, `show buffers`, and the VPP stats segment together, rather than in isolation.
* Explain the relationship between NIC/driver RX queues, VPP worker threads, and RX queue-to-worker assignment (`show interface rx-placement`, `set interface rx-placement`).
* Identify at least one concrete bottleneck (e.g. FFI call overhead, single-worker saturation, buffer starvation, NUMA mismatch) using measurements rather than guesswork, and propose (and ideally implement) a fix.
* Distinguish symptoms that point to the **node itself** (e.g. `packet_classify` cost) from symptoms that point to **the environment** (e.g. CPU pinning, hyperthreading, driver queue depth) — Section 4.4.1 of the guide is directly relevant here.
* Present a performance investigation as a report: hypothesis → measurement → conclusion, not just a screenshot dump.

---

## Prerequisites

* Completed and reviewed `task_03.md` — a working `rust_classify_plugin` node, wired into the graph, with counters and trace formatting.
* VPP built from source, debug and release (Section 2 of the guide).
* Comfortable with `vppctl`, `trace add`, `pcap trace`, `packet-generator`, `show errors`, `show run` (Sections 4.1–4.3).
* Comfortable with the performance environment guidance in Section 4.4.1 (CPU isolation, `taskset`, NUMA) — you will actually apply it this time, not just read about it.
* Basic Linux networking: network namespaces, veth pairs, `tc`, `ethtool` (used to inspect/tune NIC ring buffers if you have access to a real NIC).
* A machine (or a pair of VMs) where you're allowed to isolate CPU cores and, ideally, run as root — this assignment needs a bit more control over the host than task_03 did.

---

## Solution Architecture

```text
        Load generator                         VPP + rust_classify_plugin              Sink / responder
        (iperf3 / trex /                        (worker thread(s), pinned,              (echo server /
         custom flooder)                         rx-placement configured)               vpp_test_server /
              |                                          |                               plain netcat/iperf3)
              |  veth0 / physical NIC                    |   veth1 / physical NIC              |
              +----------------- namespace A ----VPP-----+---------- namespace B --------------+
                                                  |
                                     +------------+-------------+
                                     |  rust-classify-node       |
                                     |  -> packet_classify (Rust)|
                                     |  -> forward / error-drop  |
                                     +------------+-------------+
                                                  |
                                     +------------+-------------+
                                     |  Observability plane       |
                                     |  show run / show errors    |
                                     |  stats segment (Section 7.2)|
                                     |  show hardware-interfaces  |
                                     |  show interface rx-placement|
                                     |  perf / Hotspot             |
                                     +----------------------------+
```

Two acceptable topology variants — pick whichever is realistic given your hardware, and say why in your report:

* **Variant 1 — veth-based, single host** (same pattern as Section 6.2 of the guide): two Linux network namespaces connected via a veth pair (or two veth pairs with VPP bridging/routing between them), VPP with host-interfaces on both sides, your node in the forwarding path. Lower absolute throughput ceiling, but fully reproducible on a laptop/VM.
* **Variant 2 — two physical/virtual NICs, two machines**: closer to the "two machines with real NICs" recommendation in Section 4.4.1, giving you cleaner numbers and removing the load generator's own CPU usage from your VPP host's measurements. Requires more hardware/lab access.

Either way, **your node must be on the actual forwarding path of real, externally generated packets** — this is the key difference from task_03's Part C, where all traffic came from `packet-generator` running inside the same VPP instance.

---

## Part A: Building the bench

1. Choose and set up your topology (Variant 1 or 2 above). Document the exact commands used (namespaces/veth creation, IP addressing, VPP config files) in a `bench/` directory — this must be scriptable and re-runnable, not a one-off manual setup.
2. Wire your `rust_classify_plugin` node into the actual forwarding path (not just a `packet-generator` test graph as in task_03 Part A): incoming UDP traffic on the "outside" interface should hit your node before being forwarded toward the sink.
3. Stand up a minimal sink on the other side that can actually respond to traffic — this can be:
   * the VPP built-in echo server (`test echo server`, Section 4.4.2), or
   * a plain `iperf3 -s` / `nc -ul` listener if you only care about one-directional throughput and counters, not RTT.
4. Sanity check the bench with a **small, manual** amount of traffic first (e.g. a handful of `ping`/`iperf3 -t 2` packets) and confirm via `show trace` that packets are actually reaching and passing through your node — do not proceed to load generation until this is confirmed.
5. Write down, in your own words, what "zero load" baseline looks like: `show run` cycles/vector for your node with the interface up but idle, `show errors` all at zero, `show hardware-interfaces` counters. This baseline is what every later measurement will be compared against.

## Part B: Generating load

6. Generate load against the bench using **at least two different tools/approaches**, to cross-check each other. Suggested combinations (pick what fits your topology and time budget):
   * `iperf3` in UDP mode (`-u -b <rate>`) for controlled, fixed-bitrate load with built-in loss/jitter reporting on the receiving side.
   * A small custom flooder (Rust, using the `SO_REUSEPORT` + `recvmmsg`/batched-send patterns from task_02, or a simple Scapy/`socket` script) if you want packets shaped exactly like the ones your node classifies (valid UDP, IPv4-with-options, invalid EtherType, truncated payload) rather than iperf3's generic UDP stream.
   * `t-rex` (TRex traffic generator) if you have access to it and a spare NIC — optional, but gives you PPS numbers well beyond what a software flooder on the same host can produce.
7. Run load at **multiple distinct levels** (e.g. a "light" level well under your bench's ceiling, a "moderate" level, and a level intended to actually saturate something) rather than a single all-out test — you need at least 3 data points to say anything about how a metric scales with load, not just what happens at the extreme.
8. For each load level, capture the following in a consistent, scripted way (write a small script that snapshots all of this at once — it will need to run identically for every level, so don't do this by hand each time):
   * `show run` (cycles/vector per node, including yours).
   * `show errors` (your `malformed_packet` / `unsupported_protocol` / `forwarded_ok` counters, plus any interface/driver drop counters).
   * `show hardware-interfaces` (RX/TX packet and byte counters, and any hardware-reported drops/errors).
   * The load generator's own reported numbers (achieved PPS/bandwidth, loss %, latency if available) — you need the "offered load" side, not just what VPP saw, to reason about drops that happen **before** VPP (e.g. kernel/driver level, if you're not using a fully userspace/DPDK NIC).
9. Repeat the same load sequence with your node's classification logic effectively disabled (e.g. a build-time or runtime flag that makes the node act as pure passthrough, skipping the `packet_classify` call) to isolate **the cost of your FFI call specifically** from the cost of "having any custom node at all" in the graph. Compare `show run` cycles/vector between the two variants at the same load level.

## Part C: Queues, placement, and NUMA

10. Inspect RX queue → worker assignment with `show interface rx-placement`. If you only have a single worker, deliberately configure at least 2 workers (`cpu { workers N }`, Section 4.4.1) and multiple RX queues on your interface(s) (`ethtool -L`/driver-dependent, or `num-rx-queues` in VPP's interface creation, depending on the driver you're using), then use `set interface rx-placement` to control which worker services which queue.
11. Re-run one of your Part B load levels with:
    * a single worker (baseline), and
    * multiple workers with RX queues spread across them,

    and compare `show run` per-worker cycles/vector and your node's counters. Explain whether/why throughput or drop rate changed, referencing Section 1's "lock-free" design and Section 4.4.1's NUMA guidance from the reference guide.
12. If your hardware has more than one NUMA node, deliberately create a **NUMA-mismatched** configuration once (worker pinned to a core on one socket, NIC local to the other — Section 4.4.1 describes exactly this scenario) and measure the effect. If your setup only has a single NUMA node, note this explicitly in your report and explain, from the guide's description, what you would expect to observe and why, rather than fabricating numbers you cannot measure.
13. Apply the CPU isolation guidance from Section 4.4.1 (`isolcpus`/`taskset`, `main-core`/`corelist-workers`) if you have not already, and re-run your highest load level once more, comparing `context-switches` (via `perf stat`) before and after isolation.

## Part D: Bottleneck analysis and report

14. Based on everything gathered in Parts B and C, state **one specific bottleneck hypothesis** for your bench (e.g. "single worker thread saturates before the NIC does", "the FFI call itself is a small fraction of cycles/vector and the real cost is buffer copy on the sink side", "RX queue depth is too shallow and packets are dropped by the driver before reaching VPP at all").
15. Use `perf record`/`perf stat` (Section 4.4.4) focused on your node and/or the worker thread(s) to confirm or refute the hypothesis. Include a flame graph (Hotspot) covering at least your highest-load run.
16. Attempt at least one concrete fix or mitigation informed by the measurements (e.g. add a worker, change rx-placement, increase queue depth, batch counter increments differently, adjust NIC ring buffer size via `ethtool -G` if applicable) and re-measure to show whether it helped, hurt, or made no measurable difference. A negative result, honestly reported and explained, is an acceptable outcome — a fabricated improvement is not.
17. Write up a short **Performance Report** (Markdown, in `bench/REPORT.md`) containing:
    * Bench topology description and how to reproduce it (scripts referenced, not pasted in full).
    * A table of load level → offered PPS/bandwidth → `show run` cycles/vector for your node → error counters → loss/latency as reported by the generator.
    * The passthrough-vs-classifying comparison from step 9.
    * The single-worker-vs-multi-worker (and, if applicable, NUMA-matched-vs-mismatched) comparison from Part C.
    * Your bottleneck hypothesis, the evidence for/against it, and the outcome of your attempted fix.
    * Honest "known limitations" of your bench (e.g. load generator sharing a CPU with VPP, virtualized NIC, no hardware timestamping) — a small, honestly-scoped bench is worth more than an overstated one.

---

## Code Quality Requirements

Everything from task_03 still applies to any code touched in this assignment (`rust_classify_plugin`, `network_parser`):

```bash
cargo fmt --check
cargo clippy
cargo test
```

In addition:

* All bench setup (namespace/veth creation, VPP configs, load-generation invocations, metrics snapshotting) must live in scripts under `bench/`, not in ad-hoc shell history — another student (or your mentor) should be able to reproduce your numbers by running your scripts, not by re-reading a transcript of commands you ran once.
* Any custom flooder/load-generation code you write should follow the same standards as task_02 where applicable (no unnecessary heap allocations in a tight send loop, documented `unsafe` if you use raw syscalls).
* Every metrics-snapshot script should be idempotent and safe to run repeatedly without manual cleanup between runs.

---

## Submission Structure (Pull Request)

Same workflow as task_01/task_03 — work in the provided repository, on your own branch:

```bash
git checkout -b feature/<your-name>/week3-vpp-rust-bench
```

### PR Title

```text
[Week 3] Local VPP + Rust test bench, load generation, and performance report
```

### PR Description

```markdown
## Summary
Briefly describe the bench topology and what was measured.

## Bench
- Topology (Variant 1 / Variant 2), and why
- How to reproduce (reference to bench/ scripts)

## Load Generation
- Tools used, and why more than one
- Load levels tested

## Metrics Captured
- show run / show errors / show hardware-interfaces
- rx-placement / worker configuration
- perf / Hotspot artifacts (attach or link)

## Bottleneck Analysis
- Hypothesis
- Evidence (measurements)
- Fix attempted and outcome (including negative results)

## Report
Link to bench/REPORT.md

## Known Limitations
```

---

## Pre-Review Checklist

### Bench

* [ ] Topology is scripted and reproducible from `bench/`, not manual-only.
* [ ] Real, externally generated traffic (not `packet-generator`) flows through the node.
* [ ] A documented zero-load baseline exists.

### Load & Measurement

* [ ] At least two different load-generation approaches were used.
* [ ] At least three distinct load levels were measured.
* [ ] `show run`, `show errors`, `show hardware-interfaces` were captured consistently at each level via a script.
* [ ] Passthrough-vs-classifying comparison (FFI cost isolation) is included.

### Queues, Placement, NUMA

* [ ] `show interface rx-placement` was inspected and, where possible, reconfigured.
* [ ] Single-worker vs. multi-worker comparison is included.
* [ ] NUMA behavior is either measured or, if hardware doesn't allow it, explicitly and honestly addressed.
* [ ] CPU isolation (`isolcpus`/`taskset`) was applied and its effect on `context-switches` measured.

### Analysis & Report

* [ ] A specific, falsifiable bottleneck hypothesis is stated.
* [ ] `perf`/Hotspot evidence supports or refutes the hypothesis.
* [ ] At least one fix/mitigation was attempted and honestly evaluated.
* [ ] `bench/REPORT.md` is complete and matches the structure above.

### Quality

* [ ] `cargo fmt --check`, `cargo clippy`, `cargo test` still pass for any Rust code touched.
* [ ] Bench scripts are idempotent and documented.
* [ ] Every `unsafe` block introduced or modified in this assignment is documented per the standard from task_01/task_03.

---

## Definition of Done

* [ ] A scripted, reproducible local bench exists with real traffic flowing through `rust_classify_plugin`.
* [ ] Load was generated with at least two tools across at least three load levels.
* [ ] `show run`/`show errors`/`show hardware-interfaces`/stats-segment data was captured consistently across all levels.
* [ ] RX queue-to-worker placement and (where applicable) NUMA effects were investigated.
* [ ] A specific bottleneck was identified, investigated with `perf`/Hotspot, and at least one fix was attempted and evaluated.
* [ ] `bench/REPORT.md` presents the full hypothesis → measurement → conclusion narrative.
* [ ] The PR is opened against `main`, reviewed by the mentor, all comments resolved, and merged.

---

## Key Principle

Task_03 proved that the FFI boundary and the node logic *work*. This assignment asks a different question: **under real, varying load, where does time and throughput actually go — the Rust call, the C dispatch loop, the NIC queue, the worker/core assignment, or somewhere in the Linux network stack before VPP ever sees a packet?**

> **A single number from one run proves nothing. A trend across load levels, cross-checked against a second tool and a passthrough baseline, proves something.**
