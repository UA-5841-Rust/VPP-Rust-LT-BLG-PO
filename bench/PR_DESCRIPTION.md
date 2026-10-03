Suggested title: **[Week 4] Local VPP + Rust test bench, load generation, and performance report**

## Summary

Adds a reproducible namespace/veth bench that forwards externally generated UDP
through the Rust classifier to a separate sink. Runtime passthrough keeps the
same graph while skipping the Rust call. Final measured results identify sink
UDP receive-buffer drops and AF_PACKET/kernel processing as the dominant
environment bottleneck; a repeated buffer-size mitigation did not consistently
improve loss.

## Bench

- Variant 1: two Linux namespaces, two veth pairs, VPP AF_PACKET in the middle.
- Reproduce with `bench/build.sh`, `bench/validate.sh`, `bench/matrix.sh`,
  `bench/mitigation.sh`; prerequisites and commands in `bench/README.md`.
- Preserves week3 echo behavior; adds configurable egress and immutable
  per-interface mode configuration changed under the VPP worker barrier.
- Sanity trace and sink assertions gate load generation; idle baseline saved.

## Load Generation

- iperf3 UDP: 1, 50, 1000 Mbit/s, 512-byte application payload.
- Independent eight-flow Python UDP generator: 1k, 20k, 200k PPS.
- Ten seconds per point in classify and passthrough; achieved send rates and
  sink reception/loss are preserved in JSON.

## Metrics Captured

- Before/after `show run`, errors, hardware/interface counters, buffers, stats
  segment, RX placement, thread affinity, veth/namespace/softnet/ethtool counters.
- Sink `/proc/net/snmp` UDP counters localize receive-buffer loss.
- Perf stat: task-clock, context switches, migrations, cycles, instructions.
- Highest-load CPU-clock perf recording, report, folded stacks and SVG flame
  graph in `bench/results/final-iperf-classify-1000000000/`; raw `perf.data`
  remains local for Hotspot and is ignored by Git.
- One vs two workers with explicit RX queue placement; pinned vs widened CPU
  affinity. One NUMA node, so physical-NIC NUMA mismatch is not claimed.

## Bottleneck Analysis

- High-load classifying node: ~44–45 clocks/vector; passthrough: ~26–28.
- The 1 Gbit/s classify run's 5350 losses exactly match sink RcvbufErrors,
  with zero classifier drops. AF_PACKET TX dominates inclusive sampled stacks.
- Adding workers did not beat the one-worker/one-queue baseline.
- Doubling actual receive buffer size was tested in three alternating pairs:
  no consistent improvement; median loss worsened. Negative result retained.

## Report

See [bench/REPORT.md](bench/REPORT.md) and machine-readable
[bench/results/measurements.csv](bench/results/measurements.csv).

## Validation

- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test --all-targets`: all 28 tests passed.
- Release/debug C builds passed with `-Werror`; Bash/Python syntax checks passed.
- External raw-frame integration confirmed exact unchanged delivery of valid
  UDP/IPv4-options packets, malformed/unsupported drops, and passthrough delivery
  of all six fixtures, with mode/egress traces and counters.

## Known Limitations

WSL2 laptop shared by generator, VPP and sink; virtual interfaces and one shared
TX queue; main matrix has one observation per point. CPU affinity does not
provide exclusive host-core isolation. NIC/NUMA experiments are constrained by
the available topology; no hardware timestamps or RTT claims. The measured
passthrough difference includes parsing and dispatch, not ABI overhead alone.
