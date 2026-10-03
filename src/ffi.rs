//! Allocation-free C ABI. No pointer or borrowed data escapes a call.

use crate::{parse_packet, ParseError};

/// Stable error numbers shared with `include/network_parser.h`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifyError {
    /// Complete, structurally valid UDP datagram.
    Ok = 0,
    /// Null pointer with nonzero length.
    NullPointer = 1,
    /// Incomplete header (also returned for empty input).
    PacketTooShort = 2,
    /// Unsupported Ethernet type, including VLAN and IPv6.
    InvalidEtherType = 3,
    /// IPv4 version mismatch.
    InvalidIpv4Version = 4,
    /// IPv4 IHL below five.
    InvalidIpv4HeaderLength = 5,
    /// Inconsistent IPv4 total length.
    InvalidIpv4TotalLength = 6,
    /// Inconsistent UDP length.
    InvalidUdpLength = 7,
    /// Non-UDP IP protocol.
    UnsupportedProtocol = 8,
    /// Other malformed structure.
    MalformedPacket = 9,
    /// Length cannot be represented by a Rust slice.
    InvalidLength = 10,
    /// Fragment reassembly is unsupported.
    UnsupportedFragment = 11,
}

impl From<ParseError> for ClassifyError {
    fn from(error: ParseError) -> Self {
        match error {
            ParseError::PacketTooShort => Self::PacketTooShort,
            ParseError::InvalidEtherType => Self::InvalidEtherType,
            ParseError::InvalidIpv4Version => Self::InvalidIpv4Version,
            ParseError::InvalidIpv4HeaderLength => Self::InvalidIpv4HeaderLength,
            ParseError::InvalidIpv4TotalLength => Self::InvalidIpv4TotalLength,
            ParseError::InvalidUdpLength => Self::InvalidUdpLength,
            ParseError::UnsupportedProtocol => Self::UnsupportedProtocol,
            ParseError::MalformedPacket => Self::MalformedPacket,
            ParseError::UnsupportedFragment => Self::UnsupportedFragment,
        }
    }
}

/// Result returned by value; integers use host byte order.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassifyResult {
    /// True only for a complete supported UDP datagram; checksums are not checked.
    pub is_valid: bool,
    /// Classifier identifier: 0 = unknown, 1 = UDP (not IP protocol number 17).
    pub protocol: u8,
    /// UDP destination port; zero on every failure.
    pub dest_port: u16,
    /// A stable [`ClassifyError`] value; zero means success.
    pub error_code: u32,
}

impl ClassifyResult {
    fn failure(error: ClassifyError) -> Self {
        Self {
            is_valid: false,
            protocol: 0,
            dest_port: 0,
            error_code: error as u32,
        }
    }
}

/// Classifies a complete Ethernet II frame without copying payload or allocating.
pub fn classify(data: &[u8]) -> ClassifyResult {
    match parse_packet(data) {
        Ok(packet) => ClassifyResult {
            is_valid: true,
            protocol: 1,
            dest_port: packet.udp.destination_port,
            error_code: ClassifyError::Ok as u32,
        },
        Err(error) => ClassifyResult::failure(error.into()),
    }
}

/// Reads caller-owned bytes for this call only and returns a value.
/// Empty input (including null + zero) and null pointers are handled explicitly.
///
/// # Safety
/// For non-null `data` and `0 < len <= isize::MAX`, the caller must provide
/// `len` initialized, readable bytes in ONE live allocation. That allocation
/// must remain alive and unmodified throughout the call, including by other
/// threads. The frame begins at its Ethernet header. No alignment beyond that
/// of `u8` is required. Null/oversized/zero-length inputs are rejected before
/// dereference. A non-null address alone does not prove allocation validity.
#[no_mangle]
pub unsafe extern "C" fn packet_classify(data: *const u8, len: usize) -> ClassifyResult {
    if len == 0 {
        return ClassifyResult::failure(ClassifyError::PacketTooShort);
    }
    if data.is_null() {
        return ClassifyResult::failure(ClassifyError::NullPointer);
    }
    if len > isize::MAX as usize {
        return ClassifyResult::failure(ClassifyError::InvalidLength);
    }
    // SAFETY: null, zero, and oversized lengths were checked above. The caller
    // guarantees one live initialized allocation of at least len bytes, with
    // no concurrent mutation. Only a temporary shared slice is constructed;
    // safe parse_packet validates header lengths before accessing fields.
    classify(unsafe { std::slice::from_raw_parts(data, len) })
}
