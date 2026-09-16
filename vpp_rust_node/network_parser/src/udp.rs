use crate::error::ParseError;

pub const UDP_HEADER_LEN: usize = 8;

#[derive(Debug, PartialEq)]
pub struct UdpPacket<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: &'a [u8],
}

pub fn parse_udp(data: &[u8]) -> Result<UdpPacket<'_>, ParseError> {
    if data.len() < UDP_HEADER_LEN {
        return Err(ParseError::PacketTooShort);
    }

    let source_port = u16::from_be_bytes([data[0], data[1]]);
    let destination_port = u16::from_be_bytes([data[2], data[3]]);
    let length = u16::from_be_bytes([data[4], data[5]]);
    let checksum = u16::from_be_bytes([data[6], data[7]]);

    if (length as usize) < UDP_HEADER_LEN {
        return Err(ParseError::InvalidUdpLength);
    }

    if data.len() < (length as usize) {
        return Err(ParseError::PacketTooShort);
    }

    let payload = &data[UDP_HEADER_LEN..(length as usize)];

    Ok(UdpPacket {
        source_port,
        destination_port,
        length,
        checksum,
        payload,
    })
}
