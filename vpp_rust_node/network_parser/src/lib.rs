pub mod error;
pub mod ethernet;
pub mod ffi;
pub mod ipv4;
pub mod udp;

pub use error::ParseError;

use crate::{
    ethernet::{EthernetHeader, parse_ethernet},
    ipv4::{Ipv4Header, parse_ipv4},
    udp::{UdpPacket, parse_udp},
};

#[derive(Debug, PartialEq)]
pub struct Packet<'a> {
    pub ethernet: EthernetHeader<'a>,
    pub ipv4: Option<Ipv4Header>,
    pub udp: Option<UdpPacket<'a>>,
}

pub fn parse_packet<'a>(data: &'a [u8]) -> Result<Packet<'a>, ParseError> {
    let (ethernet, post_ethparse_data) = parse_ethernet(data)?;
    let (ipv4, post_ipv4parse_data) = parse_ipv4(post_ethparse_data)?;

    if ipv4.protocol != 17 {
        return Err(ParseError::UnsupportedProtocol);
    }

    let udp = parse_udp(post_ipv4parse_data)?;

    Ok(Packet {
        ethernet,
        ipv4: Some(ipv4),
        udp: Some(udp),
    })
}
