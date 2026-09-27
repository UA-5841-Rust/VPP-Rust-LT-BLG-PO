use crate::error::ParseError;

#[derive(Debug, PartialEq)]
pub struct Ipv4Header {
    pub version: u8,
    pub ihl: u8,
    pub dscp_ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags_fragment_offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub header_checksum: u16,
    pub source_address: [u8; 4],
    pub destination_address: [u8; 4],
}

impl Ipv4Header {
    pub fn parse(data: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        if data.len() < 20 {
            return Err(ParseError::PacketTooShort);
        }

        let version_ihl = data[0];
        let version = version_ihl >> 4;
        let ihl = version_ihl & 0x0F;

        if version != 4 {
            return Err(ParseError::InvalidIpv4Version);
        }

        if ihl < 5 {
            return Err(ParseError::InvalidIpv4HeaderLength);
        }

        let header_len = (ihl as usize) * 4;

        if data.len() < header_len {
            return Err(ParseError::PacketTooShort);
        }

        let total_length = u16::from_be_bytes(data[2..4].try_into().unwrap());

        if total_length < (header_len as u16) {
            return Err(ParseError::InvalidIpv4TotalLength);
        }

        let total_len_usize = total_length as usize;

        if data.len() < total_len_usize {
            return Err(ParseError::PacketTooShort);
        }

        let dscp_ecn = data[1];
        // Safe to unwrap: data length is already verified to be >= header_len
        let identification = u16::from_be_bytes(data[4..6].try_into().unwrap());
        let flags_fragment_offset = u16::from_be_bytes(data[6..8].try_into().unwrap());
        let ttl = data[8];
        let protocol = data[9];
        let header_checksum = u16::from_be_bytes(data[10..12].try_into().unwrap());
        let source_address: [u8; 4] = data[12..16].try_into().unwrap();
        let destination_address: [u8; 4] = data[16..20].try_into().unwrap();

        let header = Self {
            version,
            ihl,
            dscp_ecn,
            total_length,
            identification,
            flags_fragment_offset,
            ttl,
            protocol,
            header_checksum,
            source_address,
            destination_address,
        };

        let remaining_data = &data[header_len..total_len_usize];

        Ok((header, remaining_data))
    }
}
