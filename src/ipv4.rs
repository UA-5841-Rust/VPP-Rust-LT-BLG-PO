use crate::ParseError;

/// Size of an IPv4 header without options.
pub const MIN_HEADER_LEN: usize = 20;

/// A parsed IPv4 header.
///
/// `options`, when present, borrows directly from the input buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Header<'a> {
    /// IP version; always `4` for a successfully parsed header.
    pub version: u8,
    /// Internet Header Length, measured in four-byte words.
    pub ihl: u8,
    /// Differentiated Services Code Point and Explicit Congestion Notification.
    pub dscp_ecn: u8,
    /// Total IPv4 packet length, including header and payload.
    pub total_length: u16,
    /// IPv4 identification field.
    pub identification: u16,
    /// IPv4 flags and fragment offset field.
    pub flags_fragment_offset: u16,
    /// Time to live.
    pub ttl: u8,
    /// Encapsulated protocol number.
    pub protocol: u8,
    /// IPv4 header checksum.
    pub header_checksum: u16,
    /// Source IPv4 address.
    pub source: [u8; 4],
    /// Destination IPv4 address.
    pub destination: [u8; 4],
    /// IPv4 options, if IHL exceeds five words.
    pub options: Option<&'a [u8]>,
}

/// Parses and validates an IPv4 header at the beginning of `data`.
///
/// The slice must contain the complete IPv4 packet declared by its total-length
/// field. Options borrow from `data` and are never copied.
pub fn parse_ipv4(data: &[u8]) -> Result<Ipv4Header<'_>, ParseError> {
    if data.is_empty() {
        return Err(ParseError::PacketTooShort);
    }

    let version = data[0] >> 4;
    let ihl = data[0] & 0x0f;
    if version != 4 {
        return Err(ParseError::InvalidIpv4Version);
    }
    if ihl < 5 {
        return Err(ParseError::InvalidIpv4HeaderLength);
    }

    let header_len = usize::from(ihl) * 4;
    if data.len() < header_len {
        return Err(ParseError::PacketTooShort);
    }

    let total_length = u16::from_be_bytes([data[2], data[3]]);
    let total_length_usize = usize::from(total_length);
    if total_length_usize < header_len || total_length_usize > data.len() {
        return Err(ParseError::InvalidIpv4TotalLength);
    }

    let mut source = [0_u8; 4];
    let mut destination = [0_u8; 4];
    source.copy_from_slice(&data[12..16]);
    destination.copy_from_slice(&data[16..20]);

    let options = if header_len > MIN_HEADER_LEN {
        Some(&data[MIN_HEADER_LEN..header_len])
    } else {
        None
    };

    Ok(Ipv4Header {
        version,
        ihl,
        dscp_ecn: data[1],
        total_length,
        identification: u16::from_be_bytes([data[4], data[5]]),
        flags_fragment_offset: u16::from_be_bytes([data[6], data[7]]),
        ttl: data[8],
        protocol: data[9],
        header_checksum: u16::from_be_bytes([data[10], data[11]]),
        source,
        destination,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_options_without_copying() {
        let mut packet = vec![0_u8; 24];
        packet[0] = 0x46;
        packet[2..4].copy_from_slice(&24_u16.to_be_bytes());
        packet[20..24].copy_from_slice(&[1, 2, 3, 4]);

        let ipv4 = parse_ipv4(&packet).unwrap();
        let options = ipv4.options.unwrap();
        assert_eq!(options, [1, 2, 3, 4]);
        assert_eq!(options.as_ptr(), packet[20..].as_ptr());
    }

    #[test]
    fn rejects_a_total_length_smaller_than_the_header() {
        let mut packet = vec![0_u8; MIN_HEADER_LEN];
        packet[0] = 0x45;
        packet[2..4].copy_from_slice(&19_u16.to_be_bytes());

        assert_eq!(
            parse_ipv4(&packet).unwrap_err(),
            ParseError::InvalidIpv4TotalLength
        );
    }
}
