# Zero-Copy Network Packet Parser

This project is a **zero-copy network packet parser** designed for use within a custom VPP plugin. It implements a zero-copy Ethernet II / IPv4 / UDP packet parser in Rust, providing a safe Rust API alongside a C-compatible FFI layer.
You can read more about the implementation [here](https://github.com/UA-5841-Rust/zero-copy).

To integrate the library with our VPP plugin (which is written in **C**), a C-compatible header file acts as the binding contract. This header file has already been generated and committed to the repository at `include/network_parser.h`. 
However, it can be re-generated at any time, and the output will remain consistent.

To re-generate the header, use the following command:

```bash
make generate_header
```

*(Before running this command, ensure you have [cbindgen](https://github.com/mozilla/cbindgen) installed on your system).*

You can run the project's unit tests using either the `cargo test` or `make test` commands.
