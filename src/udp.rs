use crate::{EthernetHeader, Ipv4Header, ParseError};

/// IPv4 protocol value assigned to UDP.
pub const PROTOCOL_NUMBER: u8 = 17;
/// Number of bytes in a UDP header.
pub const HEADER_LEN: usize = 8;

/// A UDP datagram whose payload borrows directly from the input buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpPacket<'a> {
    /// Source UDP port in host byte order.
    pub source_port: u16,
    /// Destination UDP port in host byte order.
    pub destination_port: u16,
    /// Datagram length, including its eight-byte header.
    pub length: u16,
    /// UDP checksum.
    pub checksum: u16,
    /// Payload slice borrowed from the original packet.
    pub payload: &'a [u8],
}

/// A successfully parsed Ethernet II / IPv4 / UDP packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet<'a> {
    /// Ethernet II header.
    pub ethernet: EthernetHeader,
    /// IPv4 header. Its optional options slice borrows from the input.
    pub ipv4: Ipv4Header<'a>,
    /// UDP header and zero-copy payload.
    pub udp: UdpPacket<'a>,
}

/// Parses a complete UDP datagram.
///
/// `data` must contain exactly one UDP datagram, with no missing or trailing
/// bytes. The returned payload is a slice of `data`.
pub fn parse_udp(data: &[u8]) -> Result<UdpPacket<'_>, ParseError> {
    if data.len() < HEADER_LEN {
        return Err(ParseError::PacketTooShort);
    }

    let length = u16::from_be_bytes([data[4], data[5]]);
    let length_usize = usize::from(length);
    if length_usize < HEADER_LEN || length_usize != data.len() {
        return Err(ParseError::InvalidUdpLength);
    }

    Ok(UdpPacket {
        source_port: u16::from_be_bytes([data[0], data[1]]),
        destination_port: u16::from_be_bytes([data[2], data[3]]),
        length,
        checksum: u16::from_be_bytes([data[6], data[7]]),
        payload: &data[HEADER_LEN..],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_payload() {
        let mut datagram = [0_u8; HEADER_LEN];
        datagram[0..2].copy_from_slice(&53_u16.to_be_bytes());
        datagram[2..4].copy_from_slice(&5000_u16.to_be_bytes());
        datagram[4..6].copy_from_slice(&(HEADER_LEN as u16).to_be_bytes());

        let udp = parse_udp(&datagram).unwrap();
        assert_eq!(udp.source_port, 53);
        assert_eq!(udp.destination_port, 5000);
        assert!(udp.payload.is_empty());
    }

    #[test]
    fn rejects_inconsistent_length() {
        let mut datagram = [0_u8; HEADER_LEN];
        datagram[4..6].copy_from_slice(&7_u16.to_be_bytes());

        assert_eq!(
            parse_udp(&datagram).unwrap_err(),
            ParseError::InvalidUdpLength
        );
    }
}
