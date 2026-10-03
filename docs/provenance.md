# Parser provenance and implementation references

The safe parser modules (`ethernet.rs`, `ipv4.rs`, `udp.rs`, `error.rs`, `lib.rs`)
and the 14 safe parser integration tests were reused from the local task_01
repository `RUST/zero-copy`, whose HEAD at import was
`5aed083d1409ef317236a79279433e28de86054b`.

This is a self-contained copy for reproducible builds: there is no dependency
on an absolute Windows path or an unpublished branch of the previous task.
The previous repository was not modified. Its allocation-based PacketHandle
FFI was intentionally not imported. The new entry point reuses `parse_packet`;
the parser gained explicit rejection of IPv4 fragmentation.

Reference interfaces inspected while writing the plugin:

- [VPP sample node](https://github.com/FDio/vpp/blob/master/src/examples/sample-plugin/sample/node.c)
- [VPP sample feature registration](https://github.com/FDio/vpp/blob/master/src/examples/sample-plugin/sample/sample.c)
- [VPP 25.06 plugin CMake macro](https://github.com/FDio/vpp/blob/stable/2506/src/cmake/plugin.cmake)
- [VPP 25.06 packet-generator input](https://github.com/FDio/vpp/blob/stable/2506/src/vnet/pg/input.c)
- [VPP 25.06 PG test interface](https://github.com/FDio/vpp/blob/stable/2506/test/vpp_pg_interface.py)

Source/API inspection is not a substitute for compiling and running the plugin.
