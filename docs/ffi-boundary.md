# Ownership and the C/Rust boundary

The VPP `.so` contains C graph nodes and a statically linked Rust archive. VLIB
registration, buffer metadata, feature enable/disable and enqueue operations stay
in C. The library exports `packet_classify` with the C calling convention and a
`#[repr(C)]` eight-byte result matching `include/network_parser.h`.

## Data contract

1. VPP owns each packet allocation. A worker handles it exclusively during the
   synchronous FFI call. `current_data` points at the Ethernet II header when
   entered from the `device-input` arc used by this plugin.
2. C supplies `vlib_buffer_get_current(b)` and **only** `b->current_length`.
   Buffers with `VLIB_BUFFER_NEXT_PRESENT` are rejected as unsupported, even if
   the first segment appears to contain a complete datagram. No linearization
   or payload copy is performed.
3. Rust rejects zero lengths, null pointers and lengths above `isize::MAX` before
   constructing a slice. It cannot prove that a non-null pointer names readable
   memory. One initialized, live allocation of the claimed size, and absence of
   concurrent mutation, are obligations of the C caller.
4. The only production Rust unsafe block constructs a temporary `&[u8]`.
   `parse_packet` performs all structural parsing in safe Rust, checking lengths
   before accesses. Fixed header fields are decoded into small stack values;
   payload and options borrow input memory.
5. The result contains no pointers. Rust retains no reference and never frees
   VPP memory. The worker may forward or drop the buffer immediately on return.

## Result and policy

`is_valid` means structurally valid Ethernet II / IPv4 / UDP, not checksum-verified.
`protocol=1` is the classifier's UDP identifier, not IP protocol number 17.
Ports use host byte order. All failures return protocol=0 and port=0.
Error numbers are explicit and mirrored in the C header; ABI layout is tested
in Rust and checked at C compile time.

IPv6/VLAN/other EtherTypes, non-UDP IP and IPv4 fragments are unsupported.
Both MF and nonzero fragment offset trigger rejection; DF alone is allowed.
The IPv4 total length must fit the input and contain its header. UDP length must
equal the IP payload length and be at least eight. Ethernet padding after the
declared IP packet is ignored. Checksums and option contents are not validated.

## Allocation and concurrency

The classifier has no heap allocation, global mutable state, locks or retained
handles. The node uses fixed stack arrays sized to `VLIB_FRAME_SIZE`. Classification
counts accumulate locally and are applied once per category per frame, using the
current worker's VPP counters. `error-drop` increments a separate `dropped` count,
so it does not double-count manually incremented classification counters.

The usual VPP debug trace facility can allocate trace storage when enabled.
The no-allocation claim applies to normal packet classification with tracing
disabled; diagnostic tracing/formatting and VPP framework buffer management are
not claimed allocation-free. Never measure normal hot-path performance with
trace enabled. No additional allocation is introduced for packet parsing.

In bench mode (`to <egress>`), accepted frames go to `rust-classify-forward`
and then `interface-output` on the configured egress without rewriting bytes.
ARP, TCP and ICMP control traffic continues along the normal feature arc and
L2 cross-connect. Passthrough skips the Rust call; chained buffers remain
unsupported and the forwarding node retains its Ethernet length guard.
The passthrough trace's validity field is synthetic, not a parsing result.
Without an explicit egress, the original Ethernet echo swaps MAC addresses and
returns accepted frames to ingress. Neither mode is an IP router or a UDP echo
application; IP addresses, UDP ports and payload remain unchanged.

## Unsafe inventory

- `src/ffi.rs`: one `from_raw_parts` block with the contract above.
- `plugin/node.c`: C has no Rust-style unsafe blocks; comments at the FFI call
  and MAC accesses state the buffer ownership and length requirements.
- `tests/classify.rs`: FFI calls with live slices or documented rejected sentinels.
- `tests/no_alloc.rs`: `GlobalAlloc` forwards allocation/deallocation to `System`
  without changing layouts; the test calls FFI with a bounded stack buffer.
- `tests/ffi_smoke.c`: the foreign caller keeps its stack allocation live
  through every call.

The exported function does not catch panics. Its checked parsing path has no
intentional panic; release uses `panic=abort`, so an unexpected Rust bug would
terminate VPP instead of unwinding into C. Bounds tests help detect such bugs
but cannot make invalid caller pointers safe.
