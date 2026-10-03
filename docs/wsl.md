# WSL / Linux validation

This document records the Linux workflow used to build and validate the plugin.

## Build and test

Install VPP dependencies with `make install-dep`, then run from the repository:

```bash
bash scripts/check-rust.sh
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r scripts/requirements.txt
python scripts/check_fixtures.py target/release/libnetwork_parser.so
```

With VPP checked out in `~/vpp`, connect the plugin and build both variants:

```bash
bash scripts/prepare-vpp.sh ~/vpp
cd ~/vpp
make build MAKE_PARALLEL_JOBS=2
make build-release MAKE_PARALLEL_JOBS=2
```

The plugin registers `rust-classify-node` and `rust-classify-forward`. Enable it
on a test interface with `rust classify pg0` in the VPP CLI.

Run the integration tests:

```bash
make test-debug TEST=test_rust_classify MAKE_PARALLEL_JOBS=2
make test TEST=test_rust_classify MAKE_PARALLEL_JOBS=2
```

Both variants passed all three tests. The tests cover Ethernet echo forwarding,
invalid-packet drops, counters, trace output, 1,100 packets across vectors, and
feature disablement.

## Manual validation

Generate packet-generator fixtures with:

```bash
python scripts/generate_packets.py /tmp/rust-classify-packets
```

In a fresh VPP instance run `exec /tmp/rust-classify-packets/run.cli`, then use
`show errors`, `show trace`, and `show run`. The recorded counters were:

```text
forwarded_ok=2
unsupported_protocol=4
malformed_packet=5
dropped=9
chained_buffer=0
```

Runtime evidence is stored under `artifacts/`. The trace includes
`protocol 1 port 4321 valid 1 error 0` for accepted UDP frames.

## GDB

Run the C/Rust smoke test under GDB with:

```bash
gdb -q -batch -ex run -ex 'bt' --args ./target/ffi-smoke
```

It completed with `C/Rust ABI smoke test passed`, exited normally, and reported
no stack. See [validation.md](validation.md) and
[ffi-boundary.md](ffi-boundary.md) for the detailed status and safety contract.
