# Week 4: VPP + Rust local performance bench

A reproducible Linux namespace/veth bench sends external UDP traffic through
`rust-classify-node` to a real sink. It compares Rust classification with
runtime passthrough, measures three load levels with iperf3 and a custom UDP
generator, investigates workers/queues/CPU affinity, and profiles the pipeline.

Start with [bench/README.md](bench/README.md) for prerequisites and commands.
The measured findings and limitations are in [bench/REPORT.md](bench/REPORT.md).
Raw CLI, stats, generator and profiling evidence lives in `bench/results/`.
The original assignment is preserved in [docs/task4-assignment.md](docs/task4-assignment.md).

```bash
bash bench/build.sh release
bash bench/build.sh debug
sudo -E bash bench/validate.sh
sudo -E bash bench/matrix.sh my-run
sudo -E bash bench/mitigation.sh my-fix
python3 bench/summarize.py
```

The network parser, C ABI, echo behavior and tests were carried forward from
Lesia Melnyk's week3 project; see [docs/week4-provenance.md](docs/week4-provenance.md).
`src/` performs allocation-free, bounded Ethernet/IPv4/UDP parsing.
`plugin/` adds a configurable egress and passthrough mode using immutable VPP
feature data. `rust classify <ingress> to <egress> [passthrough]` enables the
bench path; `rust classify <interface> disable` removes its exact binding.
ARP/TCP/ICMP control traffic follows the L2 cross-connect. Omitting `to`
retains the original Ethernet echo behavior.

Quality checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --all-targets`, release/debug C builds with warnings as errors,
and external raw-frame classify/passthrough integration validation.

The measured machine is WSL2 with one NUMA node. CPU affinity is applied;
exclusive boot-level CPU isolation and physical-NIC NUMA mismatch are not
claimed. See the report before interpreting the throughput as hardware capacity.
