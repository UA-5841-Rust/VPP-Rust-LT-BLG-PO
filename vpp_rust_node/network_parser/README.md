# Rust for C++/Go Developers

## Practical Assignment: Zero-Copy Network Packet Parser

### Overview

The goal of this assignment is to gain practical experience with Rust by implementing a **zero-copy network packet parser**.

You will implement a Rust library capable of parsing:

* Ethernet II
* IPv4
* UDP

The library must expose a **C-compatible FFI interface** that can be used from C/C++.

The main focus is on:

* Ownership
* Borrowing
* Lifetimes
* RAII vs. Garbage Collection
* Safe and `unsafe` Rust
* Raw pointers
* Cargo and modules
* C FFI
* Zero-copy data processing

---

# Learning Objectives

After completing this assignment, you should be able to:

* Use Rust ownership and borrowing correctly.
* Understand and use lifetimes.
* Implement a zero-copy parser.
* Work with binary network protocols.
* Handle network byte order.
* Write safe Rust code around low-level operations.
* Understand when `unsafe` is required.
* Work with raw pointers.
* Build Rust libraries with Cargo.
* Expose Rust functionality through C FFI.
* Call Rust code from C/C++.
* Design and test safe FFI boundaries.

---

# Assignment

Implement a Rust library called `network_parser`.

The library must parse a raw network packet with the following structure:

```text
+-------------------+
| Ethernet II       |
| 14 bytes          |
+-------------------+
| IPv4              |
| 20+ bytes         |
+-------------------+
| UDP               |
| 8 bytes           |
+-------------------+
| Payload           |
| N bytes           |
+-------------------+
```

The parser must work directly with the input buffer.

## Important: Zero-Copy

The parser must **not copy the packet payload**.

For example:

```rust
pub struct UdpPacket<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub payload: &'a [u8],
}
```

The `payload` must reference the original input buffer.

Avoid:

```rust
let payload = data[offset..end].to_vec();
```

Prefer:

```rust
let payload = &data[offset..end];
```

---

# Protocol Requirements

## Ethernet II

Parse:

| Field           |    Size |
| --------------- | ------: |
| Destination MAC | 6 bytes |
| Source MAC      | 6 bytes |
| EtherType       | 2 bytes |

Minimum header size:

```text
14 bytes
```

The parser must support:

```text
EtherType = 0x0800
```

for IPv4.

---

## IPv4

Parse the following fields:

| Field                 |    Size |
| --------------------- | ------: |
| Version               |  4 bits |
| IHL                   |  4 bits |
| DSCP/ECN              |  1 byte |
| Total Length          | 2 bytes |
| Identification        | 2 bytes |
| Flags/Fragment Offset | 2 bytes |
| TTL                   |  1 byte |
| Protocol              |  1 byte |
| Header Checksum       | 2 bytes |
| Source Address        | 4 bytes |
| Destination Address   | 4 bytes |

The parser must support IPv4 options.

Do **not** assume that the IPv4 header is always 20 bytes.

Header size:

```text
IHL × 4
```

---

## UDP

Parse:

| Field            |    Size |
| ---------------- | ------: |
| Source Port      | 2 bytes |
| Destination Port | 2 bytes |
| Length           | 2 bytes |
| Checksum         | 2 bytes |

Minimum UDP header size:

```text
8 bytes
```

The payload must be returned as:

```rust
&[u8]
```

without copying it.

---

# Error Handling

The parser must never panic on malformed input.

Use:

```rust
Result<T, ParseError>
```

For example:

```rust
#[derive(Debug, PartialEq)]
pub enum ParseError {
    PacketTooShort,
    InvalidEtherType,
    InvalidIpv4Version,
    InvalidIpv4HeaderLength,
    InvalidIpv4TotalLength,
    InvalidUdpLength,
    UnsupportedProtocol,
}
```

You may introduce additional error types when necessary.

Avoid using:

```rust
unwrap()
expect()
panic!()
```

for input validation.

---

# Recommended Project Structure

```text
network_parser/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── ethernet.rs
│   ├── ipv4.rs
│   ├── udp.rs
│   └── ffi.rs
├── tests/
│   ├── ethernet_tests.rs
│   ├── ipv4_tests.rs
│   ├── udp_tests.rs
│   └── integration_tests.rs
└── examples/
    └── c_ffi_example.c
```

You may use a different structure if you can justify your decision during the code review.

---

# Rust API

The core parser should provide a safe API.

Example:

```rust
pub fn parse_packet(data: &[u8]) -> Result<Packet<'_>, ParseError>;
```

A possible representation:

```rust
pub struct Packet<'a> {
    pub ethernet: EthernetHeader,
    pub ipv4: Option<Ipv4Header>,
    pub udp: Option<UdpPacket<'a>>,
}
```

The exact API design is up to you.

---

# Unsafe Rust and FFI

The core parser should use **safe Rust** whenever possible.

Unsafe code should be isolated in the FFI layer.

Recommended architecture:

```text
C / C++
    |
    v
+-------------+
| FFI Layer   |
| unsafe      |
+-------------+
    |
    v
+-------------+
| Safe Rust   |
| API         |
+-------------+
    |
    v
+-------------+
| Packet      |
| Parser      |
+-------------+
```

Every `unsafe` block must have a clear justification.

---

# C FFI

Expose a C-compatible API.

For example:

```rust
#[no_mangle]
pub unsafe extern "C" fn packet_parse(
    data: *const u8,
    len: usize,
) -> *mut PacketHandle;
```

The final API design is up to you.

The FFI layer must:

1. Validate raw pointers.
2. Validate buffer length.
3. Convert the raw pointer into a Rust slice.
4. Call the safe parser.
5. Return a C-compatible result.
6. Correctly manage ownership.
7. Provide a cleanup function when Rust owns allocated resources.

---

# Testing Requirements

At minimum, implement tests for:

## Valid Packets

* Ethernet + IPv4 + UDP
* Empty UDP payload
* IPv4 header with options

## Invalid Packets

* Packet too short
* Invalid EtherType
* Invalid IPv4 version
* Invalid IPv4 header length
* Invalid IPv4 total length
* Invalid UDP length
* Truncated payload

## Zero-Copy

Add a test proving that the UDP payload references the original packet buffer.

You should be able to explain how the test demonstrates the zero-copy behavior.

---

# Code Quality

Before submitting your assignment, run:

```bash
cargo fmt --check
```

```bash
cargo clippy
```

```bash
cargo test
```

The project should compile without avoidable warnings.

Public APIs should be documented:

```rust
/// Parses an Ethernet/IPv4/UDP packet without copying its payload.
pub fn parse_packet(
    data: &[u8],
) -> Result<Packet<'_>, ParseError> {
    // ...
}
```

---

# Submission

## Important

**Do not create a new repository for this assignment.**

Use the **repository provided by your mentor**.

Each student must create a personal feature branch and submit the implementation through a **Pull Request**.

The expected workflow is:

```text
Provided Repository
        |
        v
   Create Branch
        |
        v
 Implement Assignment
        |
        v
    Push Branch
        |
        v
 Create Pull Request
        |
        v
   Code Review
        |
   +----+----+
   |         |
Changes    Approved
Required     |
   |         |
   +----->   v
          Merge
```

---

# Step 1 — Clone the Repository

Clone the repository provided by your mentor:

```bash
git clone <repository-url>
cd <repository-directory>
```

Do not create a new repository.

---

# Step 2 — Create Your Branch

Make sure you are starting from the latest `main` branch:

```bash
git checkout main
git pull origin main
```

Create a feature branch:

```bash
git checkout -b feature/<your-name>/week1-network-parser
```

Example:

```bash
git checkout -b feature/john-doe/week1-network-parser
```

Use your actual name or the branch naming convention specified by your mentor.

---

# Step 3 — Implement the Assignment

Implement the required functionality in your feature branch.

Do not modify unrelated parts of the repository.

Your changes should be limited to the scope of this assignment.

---

# Step 4 — Commit Your Changes

Use meaningful commit messages.

Good examples:

```text
feat: add Ethernet header parser
feat: implement IPv4 parser
feat: add UDP zero-copy parser
test: add malformed packet cases
feat: add C FFI interface
docs: update project documentation
```

Avoid generic commit messages such as:

```text
update
changes
fix
test
homework
stuff
```

Keep commits reasonably small and logically grouped.

---

# Step 5 — Update Your Branch Before Submission

Before creating the Pull Request, make sure your branch contains the latest changes from `main`.

```bash
git checkout main
git pull origin main
git checkout feature/<your-name>/week1-network-parser
git merge main
```

Resolve any conflicts if necessary.

Run the tests again after merging:

```bash
cargo test
```

---

# Step 6 — Push Your Branch

Push your feature branch:

```bash
git push -u origin feature/<your-name>/week1-network-parser
```

---

# Step 7 — Create a Pull Request

Create a Pull Request in the **provided repository**.

The Pull Request should target:

```text
base: main
```

and use your feature branch as the source:

```text
compare: feature/<your-name>/week1-network-parser
```

### Pull Request Title

Use:

```text
[Week 1] Implement zero-copy network packet parser
```

---

# Pull Request Description

Use the following structure:

```markdown
## Summary

Briefly describe what was implemented.

## Implemented

- Ethernet parser
- IPv4 parser
- UDP parser
- Zero-copy payload
- Error handling
- C FFI

## Testing

- `cargo test`
- `cargo fmt --check`
- `cargo clippy`

## Zero-Copy Design

Explain how the implementation avoids copying packet data.

## Unsafe Code

Explain where `unsafe` is used and why.

## FFI Ownership

Explain who owns the input buffer and any objects returned by the Rust library.

## Known Limitations

Describe any limitations or unsupported cases.

## Checklist

- [ ] Tests pass
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy` passes
- [ ] Documentation updated
- [ ] No unnecessary `unsafe`
- [ ] Zero-copy requirement satisfied
```

---

# Pull Request Checklist

Before requesting a review, verify:

### Functionality

* [ ] Ethernet parsing works.
* [ ] IPv4 parsing works.
* [ ] IPv4 options are supported.
* [ ] UDP parsing works.
* [ ] UDP payload is returned correctly.
* [ ] Malformed packets return errors.
* [ ] Parser does not panic.

### Memory

* [ ] Payload is not copied.
* [ ] No unnecessary heap allocations are used.
* [ ] Lifetimes are used correctly.
* [ ] Ownership is clearly defined.

### Safety

* [ ] Core parser uses safe Rust.
* [ ] Unsafe code is limited to the FFI boundary.
* [ ] Raw pointers are validated.
* [ ] FFI ownership rules are documented.

### Testing

* [ ] Valid packets are tested.
* [ ] Invalid packets are tested.
* [ ] Edge cases are tested.
* [ ] Zero-copy behavior is tested.

### Quality

* [ ] `cargo fmt --check` passes.
* [ ] `cargo clippy` passes.
* [ ] `cargo test` passes.
* [ ] Public APIs are documented.
* [ ] README is complete.

---

# Code Review

The mentor will review your Pull Request.

The review will focus on:

## Rust Fundamentals

* Ownership
* Borrowing
* Lifetimes
* References
* Slices
* Error handling
* Pattern matching

You should be able to explain your design decisions.

---

## Zero-Copy Design

The reviewer will verify that:

* Packet data is not unnecessarily copied.
* UDP payload references the original buffer.
* `Vec<u8>` is not used unnecessarily.
* Lifetimes correctly represent the relationship between input and parsed data.

You should be able to explain:

```rust
fn parse_packet<'a>(
    data: &'a [u8],
) -> Result<Packet<'a>, ParseError>
```

and answer:

> What happens to the parsed packet when `data` is dropped?

---

## Memory Safety

The reviewer will inspect:

* Bounds checking
* Slice creation
* Integer conversions
* Buffer length validation
* Integer overflow risks
* Lifetime correctness
* Unsafe code

---

## Unsafe Rust

For every `unsafe` block, be prepared to explain:

1. Why is `unsafe` required?
2. What assumptions does the code make?
3. How are those assumptions validated?
4. What could happen if the assumptions are violated?
5. Why can't the operation be implemented using safe Rust?

---

## FFI

The reviewer will check:

* `extern "C"`
* `#[no_mangle]`
* Raw pointer handling
* Null pointer validation
* Buffer length validation
* Ownership
* Resource cleanup
* ABI compatibility

---

# Review Process

The review follows these steps:

```text
Student
   |
   | Push feature branch
   v
Provided Repository
   |
   | Create Pull Request
   v
Mentor Review
   |
   +------------------+
   |                  |
   v                  v
Changes Required    Approved
   |                  |
   v                  v
Student Updates     Merge
   |                  |
   +-------> Review   v
                    main
```

## If Changes Are Requested

If the mentor requests changes:

1. Read all review comments.
2. Fix the requested issues.
3. Add or update tests if necessary.
4. Commit the changes to the **same branch**.
5. Push the branch again.
6. Reply to the review comments when appropriate.
7. Request another review.

**Do not create a new Pull Request for every review iteration.**

The existing Pull Request should be updated automatically when you push new commits to the same branch.

---

# Definition of Done

The assignment is complete when:

* [ ] All functional requirements are implemented.
* [ ] All required tests pass.
* [ ] Zero-copy behavior is demonstrated.
* [ ] Unsafe code is isolated and justified.
* [ ] C FFI works.
* [ ] Documentation is complete.
* [ ] `cargo fmt --check` passes.
* [ ] `cargo clippy` passes.
* [ ] `cargo test` passes.
* [ ] Pull Request is created against `main`.
* [ ] Mentor review is completed.
* [ ] All review comments are resolved.
* [ ] Pull Request is approved.
* [ ] Pull Request is merged into `main`.

---

# Key Principle

The main goal of this assignment is not simply to parse network packets.

The main goal is to understand how Rust can provide **memory safety without garbage collection** while still allowing low-level, high-performance programming.

> **Keep unsafe code small, use borrowing to avoid unnecessary copies, and make ownership explicit across the Rust/C boundary.**
