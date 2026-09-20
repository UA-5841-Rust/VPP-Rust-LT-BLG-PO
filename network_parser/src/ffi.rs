use crate::{ParseError, parse_packet};

/// Error codes returned to C callers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CParseError {
    Ok = 0,
    NullPointer = 1,
    PacketTooShort = 2,
    InvalidEtherType = 3,
    InvalidIpv4Version = 4,
    InvalidIpv4HeaderLength = 5,
    InvalidIpv4TotalLength = 6,
    InvalidUdpLength = 7,
    UnsupportedProtocol = 8,
}

impl From<ParseError> for CParseError {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::PacketTooShort => CParseError::PacketTooShort,
            ParseError::InvalidEtherType => CParseError::InvalidEtherType,
            ParseError::InvalidIpv4Version => CParseError::InvalidIpv4Version,
            ParseError::InvalidIpv4HeaderLength => CParseError::InvalidIpv4HeaderLength,
            ParseError::InvalidIpv4TotalLength => CParseError::InvalidIpv4TotalLength,
            ParseError::InvalidUdpLength => CParseError::InvalidUdpLength,
            ParseError::UnsupportedProtocol => CParseError::UnsupportedProtocol,
        }
    }
}

const PROTOCOL_UNKNOWN: u8 = 0;
const PROTOCOL_UDP: u8 = 1;

/// Result of classifying one packet. Returned by value — no heap
/// allocation, so it's cheap to return per-packet on VPP's hot path.
#[repr(C)]
pub struct ClassifyResult {
    pub is_valid: bool,
    pub protocol: u8,    // 0 = unknown, 1 = udp, ...
    pub dest_port: u16,  // valid only if protocol == 1
    pub error_code: u32, // maps to ParseError
}

/// Classifies a raw Ethernet/IPv4/UDP packet: valid UDP, or rejected with
/// a reason.
///
/// Does no allocation and does not take ownership of `data`; it only reads
/// through the pointer for the duration of the call.
///
/// # Safety
/// Caller (the VPP dispatch function) must ensure:
/// - `data` is either null, or points to at least `len` initialized,
///   readable bytes for the duration of this call (e.g. `vlib_buffer_t`'s
///   current data region — VPP guarantees that region is contiguous and
///   valid for `current_length` bytes).
/// - The memory `data` points to is not mutated concurrently from another
///   thread while this call runs.
///
/// Validated here: `data.is_null()` is checked before any dereference: a
/// null pointer returns `CParseError::NullPointer` instead of touching
/// memory. Everything past that point goes through `parse_packet`, which
/// itself bounds-checks `len` against every header/field it reads (see
/// `ethernet::parse_ethernet_header`, `ipv4::parse_ipv4_header`,
/// `udp::parse_udp_packet`) — so a short or truncated buffer is rejected
/// with a `ParseError`, never read past its end.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn packet_classify(data: *const u8, len: usize) -> ClassifyResult {
    if data.is_null() {
        return ClassifyResult {
            is_valid: false,
            // as we use 0 = unknown
            protocol: PROTOCOL_UNKNOWN,
            dest_port: 0,
            error_code: CParseError::NullPointer as u32,
        };
    }

    let input = unsafe { std::slice::from_raw_parts(data, len) };
    match parse_packet(input) {
        Ok(packet) => ClassifyResult {
            is_valid: true,
            protocol: PROTOCOL_UDP,
            dest_port: packet.udp.dest_port,
            error_code: CParseError::Ok as u32,
        },
        Err(err) => ClassifyResult {
            is_valid: false,
            protocol: PROTOCOL_UNKNOWN,
            dest_port: 0,
            error_code: CParseError::from(err) as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds Ethernet(14) + IPv4(20, no options) + UDP(8) + payload.
    fn build_packet(ether_type: u16, ip_version_ihl: u8, protocol: u8, payload: &[u8]) -> Vec<u8> {
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&[0xAA; 6]); // dest mac
        pkt.extend_from_slice(&[0xBB; 6]); // src mac
        pkt.extend_from_slice(&ether_type.to_be_bytes());

        let udp_len = 8 + payload.len();
        let total_len = 20 + udp_len;
        pkt.push(ip_version_ihl); // version(4 bits) | ihl(4 bits)
        pkt.push(0); // dscp/ecn
        pkt.extend_from_slice(&(total_len as u16).to_be_bytes());
        pkt.extend_from_slice(&0u16.to_be_bytes()); // id
        pkt.extend_from_slice(&0u16.to_be_bytes()); // flags/frag
        pkt.push(64); // ttl
        pkt.push(protocol);
        pkt.extend_from_slice(&0u16.to_be_bytes()); // ipv4 checksum
        pkt.extend_from_slice(&[192, 168, 0, 1]); // src addr
        pkt.extend_from_slice(&[192, 168, 0, 2]); // dest addr

        pkt.extend_from_slice(&12345u16.to_be_bytes()); // src port
        pkt.extend_from_slice(&53u16.to_be_bytes()); // dest port
        pkt.extend_from_slice(&(udp_len as u16).to_be_bytes());
        pkt.extend_from_slice(&0u16.to_be_bytes()); // udp checksum
        pkt.extend_from_slice(payload);

        pkt
    }

    fn classify(data: &[u8]) -> ClassifyResult {
        unsafe { packet_classify(data.as_ptr(), data.len()) }
    }

    #[test]
    fn valid_udp_packet_is_classified() {
        let pkt = build_packet(0x0800, 0x45, 17, b"hello");
        let result = classify(&pkt);
        assert!(result.is_valid);
        assert_eq!(result.protocol, PROTOCOL_UDP);
        assert_eq!(result.dest_port, 53);
        assert_eq!(result.error_code, 0);
    }

    #[test]
    fn invalid_ether_type_is_rejected() {
        let pkt = build_packet(0x86DD, 0x45, 17, b"hello"); // IPv6 EtherType
        let result = classify(&pkt);
        assert!(!result.is_valid);
        assert_eq!(result.protocol, PROTOCOL_UNKNOWN);
        assert_eq!(result.error_code, CParseError::InvalidEtherType as u32);
    }

    #[test]
    fn unsupported_l4_protocol_is_rejected() {
        let pkt = build_packet(0x0800, 0x45, 6, b"hello"); // TCP, not UDP
        let result = classify(&pkt);
        assert!(!result.is_valid);
        assert_eq!(result.error_code, CParseError::UnsupportedProtocol as u32);
    }

    #[test]
    fn ipv4_with_options_is_still_accepted() {
        // IHL = 6 -> 24-byte header; parse_packet must skip the extra 4 bytes.
        let mut pkt = build_packet(0x0800, 0x46, 17, b"hi");
        pkt.splice(14 + 20..14 + 20, [0u8; 4]); // insert 4 bytes of options
        let total_len = (pkt.len() - 14) as u16;
        pkt[14 + 2..14 + 4].copy_from_slice(&total_len.to_be_bytes());
        let result = classify(&pkt);
        assert!(result.is_valid);
        assert_eq!(result.dest_port, 53);
    }

    #[test]
    fn truncated_packet_is_rejected_not_read_out_of_bounds() {
        let pkt = build_packet(0x0800, 0x45, 17, b"hello");
        let truncated = &pkt[..pkt.len() - 3]; // cut into the payload
        let result = classify(truncated);
        assert!(!result.is_valid);
        assert_eq!(
            result.error_code,
            CParseError::InvalidIpv4TotalLength as u32
        );
    }

    #[test]
    fn invalid_udp_length_inside_valid_ipv4_is_rejected() {
        let mut pkt = build_packet(0x0800, 0x45, 17, b"hello");
        pkt[38..40].copy_from_slice(&99u16.to_be_bytes());

        let result = classify(&pkt);
        assert!(!result.is_valid);
        assert_eq!(result.error_code, CParseError::InvalidUdpLength as u32);
    }

    #[test]
    fn too_short_for_any_header_is_rejected() {
        let result = classify(&[0u8; 5]);
        assert!(!result.is_valid);
        assert_eq!(result.error_code, CParseError::PacketTooShort as u32);
    }

    #[test]
    fn null_pointer_is_rejected_without_dereferencing() {
        let result = unsafe { packet_classify(std::ptr::null(), 100) };
        assert!(!result.is_valid);
        assert_eq!(result.error_code, CParseError::NullPointer as u32);
    }
}
