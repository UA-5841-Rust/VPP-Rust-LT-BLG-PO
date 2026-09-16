use crate::error::ParseError;

pub const ETHERNET_HEADER_LEN: usize = 14;

pub const ETHERTYPE_IPV4: u16 = 0x0800;

#[derive(Debug, PartialEq)]
pub struct EthernetHeader<'a> {
    pub destination_mac: &'a [u8],
    pub source_mac: &'a [u8],
    pub ether_type: u16,
}

pub fn parse_ethernet(data: &[u8]) -> Result<(EthernetHeader<'_>, &[u8]), ParseError> {
    if data.len() < ETHERNET_HEADER_LEN {
        return Err(ParseError::PacketTooShort);
    }

    let destination_mac = &data[0..6];
    let source_mac = &data[6..12];

    let ether_type_bytes = [data[12], data[13]];
    let ether_type = u16::from_be_bytes(ether_type_bytes);

    if ether_type != ETHERTYPE_IPV4 {
        return Err(ParseError::InvalidEtherType);
    }

    let header = EthernetHeader {
        destination_mac,
        source_mac,
        ether_type,
    };

    let remaining_data = &data[ETHERNET_HEADER_LEN..];

    Ok((header, remaining_data))
}
