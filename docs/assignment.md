# VPP + Rust Integration

## Practical Assignment: Custom VPP Node with Rust-Based Packet Classification

### Duration: 1–2 weeks

### Overview

The goal of this assignment is to connect two blocks of knowledge acquired earlier:

* **[task_01](https://github.com/UA-5841-Rust/zero-copy/blob/main/README.md) / [task_02](https://github.com/UA-5841-Rust/Low-Latency-High-Throughput-UDP-Engine/blob/main/README.md)** — safe Rust, ownership/borrowing, lifetimes, zero-copy parsing, C FFI, the unsafe boundary.
* **VPP_RUST_v_03.md** — VPP architecture (graph nodes, vector processing, workers), developer tooling (`trace`, `pcap trace`, `packet-generator`, `show errors`, `perf`), testing frameworks (`make test`, HS-Test).

You will build a **VPP plugin** that adds a new node to the processing graph. This node will take a vector of buffers, and for each packet call your `network_parser` Rust library (from task_01) across an FFI boundary to classify the packet (valid UDP / invalid / other protocol), then forward or drop the packet based on the result, incrementing your own error counters along the way.

This directly follows the approach described in **Section 7.1 "VPP and Rust"** of the guide: the node itself and its dispatch function stay in C (since VLIB requires a C ABI and direct manipulation of internal structures such as `vlib_buffer_t`/`vlib_frame_t`), while Rust is used as a safe library called through an FFI wrapper — the same general pattern already used for VCL/LD_PRELOAD.

---

## Learning Objectives

After completing this assignment, you should be able to:

* Explain why VPP plugins remain `.so` files with a C ABI, and where exactly the safe/unsafe boundary sits when integrating Rust.
* Register a new graph node (`VLIB_REGISTER_NODE`) and wire it into VPP's packet-processing graph.
* Correctly pass a raw pointer + length from `vlib_buffer_t` across FFI into a Rust library without copying data.
* Use error counters (`vlib_node_increment_counter` / `.api` counters) to observe classification results.
* Test the new node using `packet-generator`, `trace add`/`show trace`, and `pcap trace`.
* Write a `make test`-based test (VppTestCase + Scapy) to verify the node's behavior.
* Explain what assumptions the `unsafe` code at the C↔Rust boundary makes and how those assumptions are validated.

---

## Prerequisites

* Completed and accepted `task_01.md` — the `network_parser` library with C FFI (Ethernet/IPv4/UDP, zero-copy payload, `ParseError`).
* VPP built from source (Section 2 of the guide), including both debug and release builds.
* Comfortable use of `vppctl`, `trace add`, `pcap trace`, `packet-generator`, `show errors` (Sections 4.1–4.3 of the guide).
* Basic familiarity with `make test` (Section 5.1 of the guide).

---

## Solution Architecture

```text
                 Ethernet/IPv4/UDP raw packet (vector of buffers)
                              |
                              v
                 +--------------------------+
                 | VPP graph node (C)       |
                 | rust-classify-node       |
                 |  - vlib_buffer_t access  |
                 |  - batching / vector loop|
                 +--------------------------+
                              |
                    raw ptr + len (no copy)
                              |
                              v
                 +--------------------------+
                 | FFI boundary (unsafe,    |
                 | isolated, validated)     |
                 +--------------------------+
                              |
                              v
                 +--------------------------+
                 | network_parser (Rust)    |
                 | packet_classify(...)     |
                 | -> #[repr(C)] result     |
                 +--------------------------+
                              |
              +---------------+----------------+
              |                                |
              v                                v
     next-node: forward                next-node: error-drop
     (e.g. ip4-lookup /                 + increment counter
      your own "passthrough" node)        (malformed / unsupported)
```

Key constraint: **the node itself and its dispatch function are not rewritten in Rust**. Rust is only responsible for parsing/classifying a byte buffer — exactly what it already did in task_01, except it is now invoked from a real VPP plugin instead of a standalone C test example.

---

## Part A (roughly days 1–3): Rust-side prep and plugin skeleton

1. Extend `network_parser` (from task_01) with a new classification FFI function that performs no unnecessary allocations:

   ```rust
   #[repr(C)]
   pub struct ClassifyResult {
       pub is_valid: bool,
       pub protocol: u8,      // 0 = unknown, 1 = udp, ...
       pub dest_port: u16,    // valid only if protocol == 1
       pub error_code: u32,   // maps to ParseError, 0 = OK
   }

   #[no_mangle]
   pub unsafe extern "C" fn packet_classify(
       data: *const u8,
       len: usize,
   ) -> ClassifyResult;
   ```

   This function should fully reuse the already-written safe `parse_packet` from task_01 — i.e. `unsafe` here only wraps `slice::from_raw_parts` and the call into the safe function.

2. Build `network_parser` as a `cdylib`/`staticlib` (`crate-type = ["staticlib", "cdylib"]` in `Cargo.toml`), and generate a C header (either by hand or via `cbindgen`).

3. Create a new VPP plugin (e.g. `rust_classify_plugin`) following the pattern of existing plugins under `src/plugins/`. At this stage the node should be a plain passthrough (all packets go to the same next node) — Rust is not yet wired in. The goal of this step is to confirm that:
   * the plugin builds together with VPP,
   * it is visible in `show plugins`,
   * the node is visible in the graph (`show node <name>` / `show run`).

4. Link the Rust static library into the plugin build (at the VPP CMake/Makefile level) and confirm that the `packet_classify` symbol resolves at link time — without calling it from the dispatch function yet.

## Part B (days 3–7): Node logic and FFI integration

5. In the node's dispatch function, for each buffer in the vector:
   * get a pointer to the current data (`vlib_buffer_get_current`) and its length (`b->current_length`);
   * call `packet_classify(data_ptr, len)`;
   * based on the returned `ClassifyResult`, decide the next node: a valid UDP packet → continue down the graph (e.g. into your own "echo" node or into `ip4-lookup`, whichever is easier to test), an invalid/unsupported packet → `error-drop`.

6. Add your own error counters following the pattern in Section 4.1 of the guide (either a `.api` `counters` block or `vlib_register_errors`), at minimum: `malformed_packet`, `unsupported_protocol`, `forwarded_ok`. Increment them **in batches** rather than per-packet wherever possible.

7. Respect the zero-allocation requirement on the hot path: no allocations either in the C dispatch function or in the Rust `packet_classify` function (it should only read the buffer and return a struct by value — no `Box`/`Vec`/`String`).

8. Add a trace-formatting function for the node (`format_trace`) that prints the parsed fields (protocol, port, validity) — this will be needed to verify behavior via `show trace`.

## Part C (days 7–10): Testing

9. Using `packet-generator` (Section 4.3), generate several packet streams:
   * a valid Ethernet+IPv4+UDP packet,
   * an IPv4 packet with options (IHL > 5),
   * a packet with an invalid EtherType,
   * a truncated UDP payload.

   Check `show errors` — the counters should match your expectations.

10. Enable `trace add <your-node> <N>`, run the traffic, and inspect the output of `show trace` — confirm that the fields parsed by the Rust code are correctly displayed in the node's trace output.

11. Optionally: capture/replay a `.pcap` file via `pcap trace` / `packet-generator ... { pcap <path> }` to test the node against real captured traffic rather than only synthetic packets.

12. Write a `make test`-style test (`VppTestCase`, Section 5.1): use Scapy to build packets covering the same cases as in task_01 (valid, with options, truncated, invalid EtherType/version/header length), add them to a stream via `add_stream`, and verify via the API/statistics that the node's counters match expectations.

13. (Recommended) Run a scenario with a truncated/deliberately corrupted packet under `make debug` + GDB (Section 4.1) to confirm firsthand that the FFI boundary does not crash on edge cases (empty buffer, zero length, `len` shorter than the minimum header size).

## Part D (optional, bonus): Performance sanity check

14. Compare `vppctl show run` (cycles/vector for your node) before and after adding the Rust call — this gives you a first sense of the cost of crossing the FFI boundary per packet (Section 7.2 of the guide specifically recommends `show run` as a lightweight first pass before reaching for `perf`).

15. Optionally, profile with `perf record` + Hotspot (Section 4.4.4), focused specifically on your node/the `packet_classify` call.

---

## Code Quality Requirements

```bash
cargo fmt --check
cargo clippy
cargo test
```

For the C side of the plugin — follow the coding style of the rest of the VPP codebase (look at neighboring plugins in `src/plugins/` as an example of formatting, naming conventions, and `VLIB_REGISTER_NODE` declarations).

Every `unsafe` block (on both the C and Rust sides) must have a comment explaining:

1. Why `unsafe` is required here.
2. What assumptions are made about the pointer/length.
3. Where and how those assumptions are validated (e.g. checking `len >= 14` before accessing the Ethernet header).

---

## Submission Structure (Pull Request)

As with task_01 — **do not create a new repository**; work in the repository provided by your mentor, using a separate branch per student:

```bash
git checkout -b feature/<your-name>/week2-vpp-rust-node
```

### PR Title

```text
[Week 2] VPP node with Rust-based packet classification
```

### PR Description

```markdown
## Summary
Briefly describe what was implemented.

## Implemented
- Rust FFI function packet_classify
- VPP graph node in C integrating Rust-based classification
- Custom error counters
- Trace format for the node
- Tests: packet-generator scenarios + make test

## Testing
- `cargo test` / `cargo fmt --check` / `cargo clippy`
- `make test TEST=<...>`
- Example `show errors` / `show trace` output (attach to the PR)

## FFI Boundary
Explain what exactly crosses the C/Rust boundary (raw ptr + len), who owns
the buffer, and why the Rust side performs no allocations.

## Unsafe Code
List the unsafe blocks on both sides and the justification for each.

## Performance Notes (optional)
show run before/after, and any perf/Hotspot observations if collected.

## Known Limitations
```

---

## Pre-Review Checklist

### Functionality
* [ ] The node is registered and visible in the VPP graph.
* [ ] Valid UDP packets are correctly classified and forwarded to the next node.
* [ ] Invalid/unsupported packets are correctly dropped.
* [ ] Error counters are incremented correctly and in batches.
* [ ] The trace format prints the parsed fields.

### Memory and Safety
* [ ] No payload copies occur when crossing the FFI boundary.
* [ ] No allocations on the hot path (neither in C nor in Rust).
* [ ] Every `unsafe` block is documented.
* [ ] Buffer length is validated **before** accessing header fields.
* [ ] The plugin does not crash on an empty/truncated input buffer (verified under GDB).

### Testing
* [ ] `packet-generator` scenarios cover both valid and invalid packets.
* [ ] A `make test` case is written and passes.
* [ ] `show trace` and `show errors` output is attached to the PR as evidence.

### Quality
* [ ] `cargo fmt --check`, `cargo clippy`, `cargo test` all pass.
* [ ] The plugin's C code style matches the rest of the VPP codebase.
* [ ] Public Rust APIs are documented.

---

## Definition of Done

* [ ] The Rust FFI function `packet_classify` is implemented and covered by tests.
* [ ] The VPP plugin builds together with the main VPP tree.
* [ ] The node is correctly wired into the graph and visible via `show run`/`show node`.
* [ ] Counters, trace, and tests (`packet-generator` + `make test`) are demonstrated.
* [ ] Every `unsafe` block is justified and discussed during review.
* [ ] The PR is opened against `main`, reviewed by the mentor, all comments resolved, and merged.

---

## Key Principle

The goal of this assignment is not to build a perfect production-grade plugin, but to gain hands-on experience with **exactly where the boundary lies between safe Rust, unsafe FFI, and VPP's C code**, and to learn to work with that boundary deliberately: no data copies, no hidden allocations on the hot path, and clearly assigned responsibility for validation on each side of the boundary.

> **Rust gives you guarantees within its own code. The FFI boundary is where those guarantees end and your personal responsibility begins.**
