# Validation status

## Verified locally on Windows, 2026-09-06

- Rust 1.97.1, Cargo 1.97.1.
- `cargo fmt --check`: passed.
- `cargo test --all-targets`: 28 tests passed (6 module, 14 reused parser,
  7 classifier/ABI, 1 allocation-count test).
- `cargo build --release`: passed; native Windows static/shared library produced.
- Scapy 2.6.1: 11 generated scenarios classified through the actual Rust DLL;
  all categories matched expectations. PCAP round trips preserved exact bytes.
- Python source syntax checks: passed.
- `cargo clippy --all-targets -- -D warnings`: could not launch. Windows
  Application Control blocked `cargo-clippy` with OS error 4551. Not a pass.

## Verified in WSL/Linux

- Linux Clippy and all Rust checks: passed.
- C-header compilation and static-link ABI smoke test: passed.
- VPP debug/release plugin compilation and dynamic loading: passed.
- Node registration, device-input wiring, packet forwarding/drop and counters: passed.
- `make test TEST=test_rust_classify` and debug variant: 3/3 tests passed each.
- Actual `show plugins`, `show node`, `show errors`, `show trace`, `show run` logs: captured in `artifacts/`.
- Debug/GDB corrupted packet and empty-buffer checks: GDB smoke test passed.
- Optional chained-buffer integration case and performance comparison.

VPP debug/release execution, Linux compilation, GDB smoke validation and runtime evidence are confirmed. Performance profiling was not performed.

## Evidence to record before PR

Record Linux distribution/kernel, VPP commit, Rust version and exact build/test
commands. Attach real output rather than replacing expected counts with an
unverified success claim. Record any changes required for the mentor's VPP
version and update the limitations in README.
