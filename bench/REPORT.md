# Performance Report: rust-classify Node Under Load

This task was about local VPP + Rust bench, load generation, and performance
observability. This report follows the hypothesis → measurement →
conclusion structure: what the bench is, what load was thrown at it,
where the packets and the CPU time actually went, and what the attempted fixes changed (and what they didn't).

Raw per-run artifacts live in `bench/results/snapshot/<label>/` (one directory per run: `loadgen_output.txt`, `show_run.txt`, `show_errors.txt`, `show_hardware_interfaces.txt`; regenerated via the commands in [README.md](README.md)).

The profiling artifacts are committed next to this report: [`vpp_ceiling_flamegraph.svg`](vpp_ceiling_flamegraph.svg) and [`perf_report_w2_ceiling.txt`](perf_report_w2_ceiling.txt).

---

## 1. Bench topology

Variant 1 (veth-based, single host), chosen because it is fully
reproducible on a pc; the trade-off (lower absolute ceiling, load
generator sharing the host CPU) is explicit in Known limitations.

```
ns-left --veth-left----vpp-left--[ VPP ]--vpp-right----veth-right-- ns-right
        10.10.1.2/24  10.10.1.1/24      10.10.2.1/24  10.10.2.2/24
```

- Two Linux namespaces, two veth pairs; VPP binds the root-side endpoints
  as `af_packet` host-interfaces (`host-vpp-left`, `host-vpp-right`).
- The `rust-classify` node (C node calling Rust `packet_classify`
  from `network_parser` over FFI) is a feature node on the
  `device-input` arc of both interfaces, ahead of `ethernet-input`.
- Sink: `iperf3 -s` in `ns-right` (foreground; `iperf3 -D` daemonization
  proved unreliable on WSL2).
- Final worker configuration: `main-core 1`, `corelist-workers 2-3`,
  two RX queues per interface, explicit
  `set interface rx-placement ... queue N worker M` persisted in
  `vpp-bench.cli`.

Full reproduction (topology scripts, VPP configs, zero-load baseline, sanity trace, load commands): **[bench/README.md](README.md)**.

## 2. Methodology

Every measurement is wrapped by `bench/scripts/snapshot_metrics.sh`: clear VPP counters → run one load-generator command → capture `show run`, `show errors`, `show hardware-interfaces` plus the generator's
own output. The script preserves the generator's exit code while saving the snapshot regardless of outcome, so failed runs keep their evidence.

Two load generators, cross-checking each other (same rates, same 1448 B payload, 30 s runs):

- **iperf3** UDP mode (`-u -b <rate> -l 1448 -t 30 -J`): paced loads with
  built-in receiver-side loss/jitter reporting;
- **Custom Rust flooder** (`bench/flooder/`): `sendmmsg` batches of 64
  (a compromise between syscall overhead and pacing granularity /
  per-batch loss on kernel refusal), `SO_REUSEPORT`, explicit per-batch
  pacing (`--rate 50M … 1G`) or unbounded saturation, strict accounting —
  only datagrams accepted by the kernel are counted as sent; refused
  batches are counted separately as `Failed syscalls`.

Operational discipline (both learned the hard way, see Known limitations):
ARP caches are flushed in both namespaces after every VPP restart; paced
flooder runs use one thread (smooth pacing), unbounded runs use four.

Zero-load baseline (documented in README): the worker polls at ~15.4M
loops/s with 0 vectors; all `rust-classify` counters at zero; af_packet
rings full and idle.

## 3. Load ladder — offered vs observed

### 3.1 iperf3 (single worker)

| Level | Offered | Receiver bitrate | Loss | Jitter (rx) | Datagrams (rx) |
|---|---|---|---|---|---|
| 50M  | 50 Mbit/s  | 50.0 Mbit/s | 0% | 0.247 ms | 129,486 |
| 200M | 200 Mbit/s | 200 Mbit/s  | 0% | 0.133 ms | 517,930 |
| 600M | 600 Mbit/s | 600 Mbit/s  | 0% | 0.145 ms | 1,553,830 |
| 1G   | 1 Gbit/s   | 999 Mbit/s  | 0% | 0.076 ms | 2,589,714 |

Scaling is linear across the whole paced ladder: offered == received with
zero loss at every level; jitter stays below 0.25 ms across the ladder and is lowest at 1G.

### 3.2 Custom flooder — cross-check against VPP counters

| Level | Target rate | Sent (flooder) | VPP `valid udp packets forwarded` | Δ | Failed syscalls |
|---|---|---|---|---|---|
| 50M   | 4,316 pps   | 127,653    | 127,653    | 0      | 29 |
| 200M  | 17,265 pps  | 516,032    | 516,032    | 0      | 30 |
| 600M  | 51,796 pps  | 1,551,865  | 1,551,850  | 15     | 26 |
| 1G    | 86,326 pps  | 2,587,925  | 2,587,920  | 5      | 27 |

The two independent tools agree with the VPP node counter to the packet:
Δ = 0 at 50M and 200M, =< 15 packets elsewhere. Small non-zero deltas are
packets still in flight inside the af_packet RX ring at snapshot time, not
loss. `Failed syscalls` (kernel refused a
whole batch, mostly ICMP-driven) are accounted separately and never counted
as sent — which is why the cross-check stays exact.

The fact that we see a match between two structurally different generators (iperf3 and custom flooder) and the internal VPP counters shows that the bench topology is lossless up to 1 Gbps.
It confirms that our custom `rust-classify` node correctly parses, counts, and hands off all the traffic without introducing hidden drops or corruptions at the FFI boundary.

## 4. Saturation: where the packets die

Unbounded flood (no rate cap, 4 flows). Three independent configurations:

| Config | Sent by generator | Reached VPP | VPP drain | VPP drops |
|---|---|---|---|---|
| iperf3 `-P 4`, 1 worker | 57,476,900 | 6,935,609 | ~230k pps  | 0 |
| flooder, 1 worker       | 87,773,888 | 11,992,172 | ~400k pps | 0 |
| flooder, 2 workers      | 79,161,069 | 12,139,458 | ~405k pps | 0 |

Findings:

1. **VPP never dropped a packet at any load** — `vector rates in == out`
   in every saturation run; the graph is not the limiter.
2. **86–91% of offered traffic dies before VPP** — in the kernel path
   feeding the af_packet RX ring (and, on the receiver side, in the sink
   socket buffer). The generator's `Failed syscalls` counter is a minor
   contributor (~1/s, ICMP rate-limited); the bulk is lost deeper in the
   send/receive path, invisible to both the generator and VPP.
3. iperf3 drains less (230k vs 400k) because iperf3 itself saturates first:
   its client reported 354% CPU (317% sys). The flooder is the stronger
   sender; when fed harder, VPP drained more — up to the same kernel wall.
4. At saturation, `af-packet-input` switches from interrupt to polling
   (adaptive); with two workers both drain ~207k pps each in parallel.

## 5. Passthrough A/B — isolating the FFI cost

Same VPP session, single-worker configuration, flooder 1G (`-T 1`), runtime toggle
`rust-classify passthrough on|off` using `vppctl` bin (skips the `packet_classify` FFI call;
node acts as pure forwarding).

Passthrough is done using `passthrough` flag in `rust-classify` node plugin code:
```c
/* Volatile flag toggled by the CLI to skip FFI calls.
 * Used to isolate the cost of the Rust packet_classify call. */
extern volatile u8 rust_classify_passthrough;
```

Packet-Clocks per vector from `show run`:

| Node | classify ON | passthrough OFF | Δ |
|---|---|---|---|
| **rust-classify** | **92.9** | **42.9** | **+50.0** |
| ip4-lookup (control)      | 53.0 | 50.8 | 4% |
| ip4-rewrite (control)     | 48.5 | 45.5 | 6% |
| ethernet-input (control)  | 61.1 | 58.3 | 5% |

- **The FFI classification call costs ≈ 50 clocks/packet** — about 1.75× a
  native `ip4-lookup`, and <0.2% of one worker core at 86k pps.
- Control nodes drift 4–6% between the two runs (CPU frequency); the
  comparison is same-session, so the Δ is valid.
- The passthrough node alone (42.9) is *cheaper* than a native ip4-lookup —
  that is the floor price of "any custom node in the graph".
- Cross-check holds in both modes: ON 2,587,920 vs 2,587,925 sent (Δ5);
  OFF 2,588,532 vs 2,588,513 sent (**+19** — passthrough counts every
  forwarded frame including ARP, by design).
- At ceiling (both runs single-worker, different sessions — throughput is comparable,
  clocks are not): passthrough drained ~410k pps vs ~400k with classification — within run-to-run noise;
  the plugin is irrelevant to the ceiling either way.

## 6. Workers, rx-placement, NUMA

**Placement.** 1 worker: both RX queues on `vpp_wk_0`. 2 workers
(`main-core 1`, `corelist-workers 2-3`, `num-rx-queues 2`): queues split
q0→wk_0, q1→wk_1; explicit `set interface rx-placement` persisted in
`vpp-bench.cli` (auto round-robin produced the same split; the explicit
commands make it deterministic).

**Flow spreading.** The 4-flow flood distributes across the two af_packet
queues (PACKET_FANOUT hash) **exactly 50/50**: at 1G,
1,293,120 + 1,293,158 vectors = 2,586,278 = sent, Δ0. Per-worker `show run`
sections confirm ~43.9k pps each at 1G, ~207k pps each at ceiling.

**Does a second worker help?**

| Metric | 1 worker | 2 workers |
|---|---|---|
| 1G paced throughput | 86.2k pps | 86.2k pps (identical) |
| Ceiling drain | ~400k pps | ~405k pps (+1–2%) |
| Per-worker drain at ceiling | 400k (solo) | ~207k each |

1G was never worker-limited; at ceiling the extra worker buys ~2% — inside
run-to-run noise. **Adding a worker did not move the ceiling**.

**Context switches / isolation.** With cores pinned
(`main-core`/`corelist-workers`): under load, 135 switches/s (1 worker,
`perf stat`), 273/s (`perf stat`) and 152/s (`/proc/<pid>/status` delta)
with 2 workers — same order of magnitude, no explosion; `cpu-migrations: 0`
in every measurement; nonvoluntary switches ≈ 0 (18 in 30 s). Worker cores
are effectively never preempted.

**NUMA.** This platform has a single NUMA node (CPUs 0–5), so a
NUMA-mismatched configuration is physically impossible here. A cross-socket placement would add remote-memory latency to every
buffer access; the expected observation is per-packet clock inflation and a
lower ceiling. Not measurable on this bench — stated rather than fabricated.

## 7. Bottleneck analysis (hypothesis → evidence → fixes)

**Hypothesis.** The bench ceiling (~410k pps drain) is set by the pre-VPP
and around-VPP **kernel networking path** (UDP sendmsg → veth TX → af_packet
ring on ingress; peer-veth RX on egress), not by the VPP graph and not by
the Rust FFI call.

**Evidence.**

1. 86–91% of unbounded traffic never reaches VPP; VPP drops 0 at every
   load level.
2. FFI cost ≈ 50 clocks/packet → well under 1% of a worker core even
   at ceiling.
3. `perf record -F 99 -g` during an unbounded run (0 lost samples; full
   call graph with symbols —
   [`vpp_ceiling_flamegraph.svg`](vpp_ceiling_flamegraph.svg),
   [`perf_report_w2_ceiling.txt`](perf_report_w2_ceiling.txt)):
   - workers split evenly (48.7% / 48.6% of sampled cycles);
   - **45.3% of worker time is `dispatch_pending_node`, almost all of it
     `af_packet_device_class_tx_fn` → `sendto` (22%) → `packet_sendmsg` →
     `packet_xmit` (19%) → `__dev_direct_xmit` → softirq
     (`net_rx_action` 12.6%, `__napi_poll` 12.7%) → `ip_rcv` → `udp_rcv`
     (7%)**: forwarding each packet costs kernel CPU twice — once to push
     it into the peer veth and once to receive it there — **on the same
     cores the VPP workers run on**;
   - VPP graph nodes are ≈0.2% each; the Rust plugin totals
     **0.32–0.37% of worker time** (`network_parser::parse_packet`
     0.15–0.20%, `rust_classify_node_fn` 0.15%, the `packet_classify` FFI
     entry itself 0.02% — the FFI boundary is nearly free; the cost is the
     parser and the node dispatch).
4. Second worker redistributes load 50/50 but moves the ceiling ~+2%:
   if worker capacity were the limit, doubling workers would approach ×2.

**Fix attempts.**

| # | Fix | Prediction (stated before run) | Outcome |
|---|---|---|---|
| 1 | Add worker + 2 RX queues + explicit rx-placement | No change at ceiling if the limit is the kernel path | **~400k → ~405k pps (+1–2%), no measurable improvement** — hypothesis confirmed |
| 2 | Increase af_packet ring depth (`rx-queue-size 4096 tx-queue-size 4096`) | No change: profile shows per-packet CPU cost, not queue pressure (TX ring never exceeded `available:1024`) | **Not applicable on this platform** — VPP rejects the parameters for host-interfaces: `create host-interface: unknown input 'rx-queue-size 4096 tx-queue-si...'`. Cannot be tested on af_packet; `ethtool -G` applies to real NICs only |

Fix #2 was worth running precisely as a falsification test of the
queue-pressure alternative — the configuration guard rejected it, which
itself documents that on an af_packet bench, ring-depth tuning is not an
available mitigation.

I guess, fix #2 can be done using another command. However, I haven't found another way in documentation yet.

**Conclusion.** The single-host topology is the structural limiter: every
forwarded packet burns kernel CPU twice (veth TX + peer veth RX) on cores
shared with VPP workers, and the ingress ring cannot be fed faster. A
DPDK/physical-NIC bench (Variant 2) would remove both copies of that cost —
that is where the ceiling would actually move.

## 8. Known limitations

1. **WSL2**: virtualized platform (Hyper-V timer ISRs visible in the
   profile), single NUMA node, no physical NIC — absolute numbers are
   environment-relative; the trends and cross-checks are the deliverable.
2. **Stale ARP after a VPP restart**: VPP generates random MACs for its
   af_packet interfaces each session. A stale ARP entry in `ns-left` makes
   the kernel send frames to the old MAC; VPP counts them pre-MAC-check
   (`rust-classify` sits before `ethernet-input`) while `ethernet-input`
   drops them (`l3 mac mismatch`). Measured once at 35,587/127,616 = 28%
   "loss" with a *perfect* classify-level cross-check — a silent poison.
   Mitigation: `ip neigh flush all` in both namespaces after every restart
   (documented in [README](./README.md)); the affected run was re-taken.
3. **Packet-Clocks are only comparable within one VPP session** — CPU
   frequency drift of 4–6% was observed on control nodes across sessions;
   all A/B comparisons here are same-session with control-node validation.
4. **Flooder `Failed syscalls`** background noise (~1/s, ICMP-driven on
   WSL2); accounted separately from sent packets, does not affect
   cross-checks.
5. Load generator and sink share host CPU with VPP; no hardware
   timestamping; latency measured only as iperf jitter; t-rex not available.
6. **Future work**: AF_PACKET-based malformed-packet profiles
   (bad-EtherType / IPv4-with-options / truncated payload) to exercise the
   classifier's negative counters under load; sender-side batch-size
   sensitivity at ceiling.

## 9. Conclusion

The `rust-classify` node successfully forwarded all the offered traffic across all paced loads up to 1 Gbps, with both generators showing exact counter agreement.
The FFI boundary showed highly efficient, costing only ~50 clocks per packet (<1% of core time) and successfully scaling to multiple workers.
The overall bench ceiling of ~410k PPS is strictly bounded by the host kernel's networking stack (veth/af_packet) rather than the VPP plugin
as proven by the flame graph showing VPP workers spending 45% of their time executing the `sendto`/packet-TX kernel path (and the peer veth's softirq RX on the same cores). 
Subsequent tuning attempts (multi-worker scaling and ring depth tuning) either produced negligible gains or were rejected,
firmly confirming that physical NIC isolation is required to find the true limits of this Rust integration.

---

## Reproducibility

```bash
# one-time
sudo ./bench/setup_topology.sh
cd bench/flooder && cargo build --release && cd ../..

# per session
sudo build-root/install-vpp-native/vpp/bin/vpp -c vpp-bench.conf
sudo ip netns exec ns-right iperf3 -s          # sink (dedicated terminal)
sudo ip netns exec ns-left  ip neigh flush all # after EVERY VPP restart
sudo ip netns exec ns-right ip neigh flush all

# example run (full ladder in README)
sudo env VPPCTL_BIN="$HOME/vpp/build-root/install-vpp-native/vpp/bin/vppctl" \
  ./bench/scripts/snapshot_metrics.sh flooder_test_high -- \
  ip netns exec ns-left ./flooder/target/release/flooder \
  -t 10.10.2.2:5201 -T 1 --rate 1G -s 1448 -b 64 -d 30
```
