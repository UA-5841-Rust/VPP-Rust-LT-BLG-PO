use crate::ParseError;

/// Number of bytes in an Ethernet II header.
pub const HEADER_LEN: usize = 14;

/// An Ethernet II header.
///
/// Header fields are stored by value because they are fixed-size metadata. The
/// variable-length UDP payload remains borrowed from the original input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthernetHeader {
    /// Destination MAC address.
    pub destination: [u8; 6],
    /// Source MAC address.
    pub source: [u8; 6],
    /// EtherType in host byte order.
    pub ethertype: u16,
}

/// Parses the Ethernet II header at the start of `data`.
pub fn parse_ethernet(data: &[u8]) -> Result<EthernetHeader, ParseError> {
    if data.len() < HEADER_LEN {
        return Err(ParseError::PacketTooShort);
    }

    let mut destination = [0_u8; 6];
    let mut source = [0_u8; 6];
    destination.copy_from_slice(&data[..6]);
    source.copy_from_slice(&data[6..12]);

    Ok(EthernetHeader {
        destination,
        source,
        ethertype: u16::from_be_bytes([data[12], data[13]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ethernet_header() {
        let mut packet = vec![0_u8; HEADER_LEN];
        packet[..6].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        packet[6..12].copy_from_slice(&[7, 8, 9, 10, 11, 12]);
        packet[12..14].copy_from_slice(&0x0800_u16.to_be_bytes());

        let ethernet = parse_ethernet(&packet).unwrap();
        assert_eq!(ethernet.destination, [1, 2, 3, 4, 5, 6]);
        assert_eq!(ethernet.source, [7, 8, 9, 10, 11, 12]);
        assert_eq!(ethernet.ethertype, 0x0800);
    }

    #[test]
    fn rejects_short_ethernet_header() {
        assert_eq!(
            parse_ethernet(&[0_u8; HEADER_LEN - 1]).unwrap_err(),
            ParseError::PacketTooShort
        );
    }
}
