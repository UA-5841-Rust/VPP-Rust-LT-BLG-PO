use crate::error::ParseError;

#[derive(Debug, PartialEq)]
pub struct EthernetHeader {
    pub destination_mac: [u8; 6],
    pub source_mac: [u8; 6],
    pub ether_type: u16,
}

impl EthernetHeader {
    pub fn parse(data: &[u8]) -> Result<(Self, &[u8]), ParseError> {
        if data.len() < 14 {
            return Err(ParseError::PacketTooShort);
        }

        // Safe to unwrap: We explicitly verified data.len() >= 14 above, ensuring that slicing 0..6, 6..12, and 12..14 will never panic or go out of bounds.
        let destination_mac: [u8; 6] = data[0..6].try_into().unwrap();
        let source_mac: [u8; 6] = data[6..12].try_into().unwrap();

        let ether_type_bytes: [u8; 2] = data[12..14].try_into().unwrap();

        let ether_type = u16::from_be_bytes(ether_type_bytes);

        if ether_type != 0x0800 {
            return Err(ParseError::InvalidEtherType);
        }

        let header = Self {
            destination_mac,
            source_mac,
            ether_type,
        };

        let remaining_data = &data[14..];

        Ok((header, remaining_data))
    }
}
