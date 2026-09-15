use crate::error::ParseError;

#[derive(Debug, PartialEq)]
pub struct Ipv4Header {
    pub version: u8,
    pub ihl: u8,
    pub dscp_ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub header_checksum: u16,
    pub source_address: [u8; 4],
    pub destination_address: [u8; 4],
}

pub fn parse_ipv4(data: &[u8]) -> Result<(Ipv4Header, &[u8]), ParseError> {
    if data.len() < 20 {
        return Err(ParseError::PacketTooShort);
    }

    let ihl = data[0] & 0x0F;
    if ihl < 5 {
        return Err(ParseError::InvalidIpv4HeaderLength);
    }
    let header_len = (ihl * 4) as usize;

    if data.len() < header_len {
        return Err(ParseError::PacketTooShort);
    }

    let version = data[0] >> 4;
    if version != 4 {
        return Err(ParseError::InvalidIpv4Version);
    }

    let total_length = u16::from_be_bytes([data[2], data[3]]);
    let dscp_ecn = data[1];
    let identification = u16::from_be_bytes([data[4], data[5]]);
    let flags = u16::from_be_bytes([data[6], data[7]]);
    let ttl = data[8];
    let protocol = data[9];
    let header_checksum = u16::from_be_bytes([data[10], data[11]]);
    let source_address = [data[12], data[13], data[14], data[15]];
    let destination_address = [data[16], data[17], data[18], data[19]];

    if (total_length as usize) < header_len {
        return Err(ParseError::InvalidIpv4TotalLength);
    }

    if data.len() < (total_length as usize) {
        return Err(ParseError::PacketTooShort);
    }

    let header = Ipv4Header {
        version,
        ihl,
        dscp_ecn,
        total_length,
        identification,
        flags,
        ttl,
        protocol,
        header_checksum,
        source_address,
        destination_address,
    };

    let remaining_data = &data[header_len..(total_length as usize)];

    Ok((header, remaining_data))
}
