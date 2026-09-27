pub mod error;
pub mod ethernet;
pub mod ffi;
pub mod ipv4;
pub mod udp;

use crate::error::ParseError;
use crate::ethernet::EthernetHeader;
use crate::ipv4::Ipv4Header;
use crate::udp::UdpPacket;

#[derive(Debug, PartialEq)]
pub struct Packet<'a> {
    pub ethernet: EthernetHeader,
    pub ipv4: Option<Ipv4Header>,
    pub udp: Option<UdpPacket<'a>>,
}

pub fn parse_packet(data: &[u8]) -> Result<Packet<'_>, ParseError> {
    let (ethernet, eth_payload) = EthernetHeader::parse(data)?;

    let (ipv4, ip_payload) = Ipv4Header::parse(eth_payload)?;

    if ipv4.protocol != 17 {
        return Err(ParseError::UnsupportedProtocol);
    }

    let udp = UdpPacket::parse(ip_payload)?;

    Ok(Packet {
        ethernet,
        ipv4: Some(ipv4),
        udp: Some(udp),
    })
}
