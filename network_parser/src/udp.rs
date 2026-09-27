use crate::error::ParseError;

#[derive(Debug, PartialEq)]
pub struct UdpPacket<'a> {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
    // The payload references the original input buffer (Zero-copy)
    pub payload: &'a [u8],
}

impl<'a> UdpPacket<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ParseError> {
        // Minimum UDP header size is 8 bytes
        if data.len() < 8 {
            return Err(ParseError::InvalidUdpLength);
        }

        // Safe to unwrap: data length is explicitly verified to be >= 8 above. No panic can occur here.
        let source_port = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let destination_port = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let length = u16::from_be_bytes(data[4..6].try_into().unwrap());
        let checksum = u16::from_be_bytes(data[6..8].try_into().unwrap());

        let len_usize = length as usize;

        // UDP length field includes the 8-byte header itself
        if len_usize < 8 {
            return Err(ParseError::InvalidUdpLength);
        }

        // Ensure the provided buffer actually contains the amount of data specified in the UDP header length field
        if data.len() < len_usize {
            return Err(ParseError::PacketTooShort);
        }

        // ZERO-COPY PAYLOAD EXTRACTION
        // We do not allocate new memory (no .to_vec()).
        // We simply create a slice referencing the original buffer.
        let payload = &data[8..len_usize];

        Ok(Self {
            source_port,
            destination_port,
            length,
            checksum,
            payload,
        })
    }
}
