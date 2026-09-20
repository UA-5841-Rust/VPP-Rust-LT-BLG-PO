use crate::ParseError;

/// Parsed EtherType field. We only care whether it's IPv4 — everything
/// else is just wrapped in `Unknown` and rejected by the parser.
#[derive(Debug, PartialEq, Eq)]
pub enum EthernetType {
    IpV4,
    Unknown(u16),
}

impl EthernetType {
    pub const IPV4_HEX: u16 = 0x0800;

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_))
    }
}

impl From<u16> for EthernetType {
    fn from(v: u16) -> Self {
        match v {
            Self::IPV4_HEX => EthernetType::IpV4,
            other => EthernetType::Unknown(other),
        }
    }
}

/// Ethernet II header: destination MAC, source MAC, EtherType.
/// No other EtherTypes besides IPv4.
#[derive(Debug, PartialEq, Eq)]
pub struct EthernetHeader {
    pub dest_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: EthernetType,
}

impl EthernetHeader {
    /// Fixed header size: 6 (dest MAC) + 6 (src MAC) + 2 (EtherType).
    pub const SIZE: usize = 14;
}

/// Parses the first 14 bytes of `data` as an Ethernet II header.
///
/// Fails if `data` is shorter than 14 bytes, or if the EtherType isn't
/// 0x0800 (IPv4) — this parser doesn't support anything else.
pub fn parse_ethernet_header(data: &[u8]) -> Result<EthernetHeader, ParseError> {
    if data.len() < EthernetHeader::SIZE {
        return Err(ParseError::PacketTooShort);
    }

    let ether_type = EthernetType::from(u16::from_be_bytes([data[12], data[13]]));
    if ether_type.is_unknown() {
        return Err(ParseError::InvalidEtherType);
    }

    Ok(EthernetHeader {
        dest_mac: [data[0], data[1], data[2], data[3], data[4], data[5]],
        src_mac: [data[6], data[7], data[8], data[9], data[10], data[11]],
        ether_type,
    })
}
