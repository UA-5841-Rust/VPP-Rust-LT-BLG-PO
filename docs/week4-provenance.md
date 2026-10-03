# Week4 source and evidence provenance

The starting repository contained only the assignment README and .gitignore.
Lesia Melnyk's local week3 project
`../week3/Custom-VPP-Node-with-Rust-Based-Packet-Classification` was used as the
prerequisite implementation (HEAD `c1c72ff9e453716a3e6e5715283703efb20adc6b`).
The Rust parser, C ABI, tests, helper scripts and historical docs were copied
without importing Git history or altering the week3 checkout.

`docs/assignment.md`, `docs/wsl.md`, `docs/validation.md`,
`docs/provenance.md` and `docs/ffi-boundary.md` describe that week3 baseline.
Their historical test claims are not new week4 results. Current bench behavior,
commands and evidence are documented in `bench/README.md` and `bench/REPORT.md`.
The exact week4 assignment is preserved in `docs/task4-assignment.md`.

New C code configures immutable per-interface egress/passthrough feature data.
CLI-owned vectors store previous configurations so mode changes remove the
exact opaque binding rather than accumulating duplicate features. CLI changes
use VPP's default worker barrier. Workers access only VPP feature data and
per-worker counters; the parser's FFI lifetime and contiguous-buffer contract
is unchanged. No new Rust unsafe blocks were introduced.

Only final validated runs are submitted. Preliminary measurements taken while
debugging mode changes were moved to the ignored `bench/superseded/` directory
and are excluded from the report. Runtime passthrough is independently tested
by forwarding malformed frames that the classifying mode drops.

All reported values come from local commands, not estimated throughput or
generated screenshots. Binary profiles remain local and ignored; folded stacks,
SVG flame graph, perf report text, and JSON/CLI/CSV evidence are submitted.
