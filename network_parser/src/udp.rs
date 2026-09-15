use crate::ParseError;

/// UDP header and payload.
///
/// `payload` references the original input buffer — it is not copied.
#[derive(Debug, PartialEq, Eq)]
pub struct UdpPacket<'a> {
    /// Source port.
    pub src_port: u16,
    /// Destination port.
    pub dest_port: u16,
    /// Length of the UDP datagram (header + payload).
    pub len: u16,
    /// UDP checksum.
    pub checksum: u16,
    /// Payload bytes. This is a view into the original buffer.
    pub payload: &'a [u8],
}

impl UdpPacket<'_> {
    /// UDP header size in bytes.
    const HEADER_SIZE: usize = 8;
}

/// Parses a UDP header + payload from `data`.
///
/// `data` should start right at the UDP header (the caller is expected
/// to have already skipped past Ethernet + IPv4). The `Length` field in
/// the header tells us where the payload ends, so `data` can be longer
/// than the actual UDP packet (e.g. it might have padding after it).
pub fn parse_udp_packet<'a>(data: &'a [u8]) -> Result<UdpPacket<'a>, ParseError> {
    if data.len() < UdpPacket::HEADER_SIZE {
        return Err(ParseError::PacketTooShort);
    }

    let src_port = u16::from_be_bytes([data[0], data[1]]);
    let dest_port = u16::from_be_bytes([data[2], data[3]]);

    let udp_len = usize::from(u16::from_be_bytes([data[4], data[5]]));
    // Length has to cover at least the header itself, and can't claim
    // more bytes than we actually have.
    if udp_len < UdpPacket::HEADER_SIZE || udp_len > data.len() {
        return Err(ParseError::InvalidUdpLength);
    }

    let checksum = u16::from_be_bytes([data[6], data[7]]);

    let payload = &data[UdpPacket::HEADER_SIZE..udp_len];

    Ok(UdpPacket {
        src_port,
        dest_port,
        len: udp_len as u16,
        checksum,
        payload,
    })
}
