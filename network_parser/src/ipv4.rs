use crate::ParseError;

const UDP_PROTOCOL: u8 = 17;

/// IPv4 header fields.
#[derive(Debug, PartialEq, Eq)]
pub struct IpV4Header {
    /// IP version (expected 4).
    pub version: u8,
    /// Header length in 32-bit words (minimum 5).
    pub ihl: u8,
    pub dscp_ecn: u8,
    /// Total packet length including header and payload.
    pub total_len: u16,
    pub id: u16,
    pub flags_fragment_offset: u16,
    pub ttl: u8,
    /// Next-layer protocol (expected 17 for UDP).
    pub protocol: u8,
    pub checksum: u16,
    pub src_addr: [u8; 4],
    pub dest_addr: [u8; 4],
}

impl IpV4Header {
    /// Minimum IPv4 header size in bytes (IHL = 5).
    const MIN_SIZE: usize = 20;
}

/// Parses an IPv4 header from `data`.
///
/// Returns the parsed header AND the actual header length in bytes
/// (IHL × 4), so the caller knows where the next layer starts.
///
/// Only UDP (protocol 17) is accepted; everything else is rejected as
/// unsupported, since this parser doesn't implement other protocols.
pub fn parse_ipv4_header(data: &[u8]) -> Result<(IpV4Header, usize), ParseError> {
    if data.len() < IpV4Header::MIN_SIZE {
        return Err(ParseError::PacketTooShort);
    }

    let version = data[0] >> 4;
    if version != 4 {
        return Err(ParseError::InvalidIpv4Version);
    }

    let ihl = data[0] & 0x0F;
    let header_len = usize::from(ihl) * 4;

    // IHL < 5 means header < 20 bytes, which is invalid by spec.
    // header_len > data.len() means the buffer got cut off mid-header.
    if header_len < 20 || data.len() < header_len {
        return Err(ParseError::InvalidIpv4HeaderLength);
    }

    let dscp_ecn = data[1];

    let total_len = u16::from_be_bytes([data[2], data[3]]);
    // Total Length has to cover at least the header itself,
    // and cannot exceed the actual buffer length we received.
    if (total_len as usize) < header_len || (total_len as usize) > data.len() {
        return Err(ParseError::InvalidIpv4TotalLength);
    }

    let id = u16::from_be_bytes([data[4], data[5]]);
    let flags_fragment_offset = u16::from_be_bytes([data[6], data[7]]);
    let ttl = data[8];

    let protocol = data[9];
    if protocol != UDP_PROTOCOL {
        return Err(ParseError::UnsupportedProtocol);
    }

    let checksum = u16::from_be_bytes([data[10], data[11]]);

    let src_addr = [data[12], data[13], data[14], data[15]];
    let dest_addr = [data[16], data[17], data[18], data[19]];

    Ok((
        IpV4Header {
            version,
            ihl,
            dscp_ecn,
            total_len,
            id,
            flags_fragment_offset,
            ttl,
            protocol,
            checksum,
            src_addr,
            dest_addr,
        },
        header_len,
    ))
}
