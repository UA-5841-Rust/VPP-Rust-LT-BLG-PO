//! A safe, zero-copy Ethernet II / IPv4 / UDP packet parser.
//!
//! Fixed-size headers are decoded into small value types. IPv4 options and the
//! UDP payload borrow from the caller's buffer, so packet data is never copied.

mod error;
pub mod ethernet;
pub mod ffi;
pub mod ipv4;
pub mod udp;

pub use error::ParseError;
pub use ethernet::EthernetHeader;
pub use ipv4::Ipv4Header;
pub use udp::{Packet, UdpPacket};

/// Parses one Ethernet II frame containing one complete IPv4 UDP datagram.
///
/// The returned [`Packet`] borrows IPv4 options and the UDP payload from
/// `data`; it cannot outlive that buffer. The parser performs bounds and
/// length checks before every slice access and does not panic for malformed
/// input.
pub fn parse_packet(data: &[u8]) -> Result<Packet<'_>, ParseError> {
    let ethernet = ethernet::parse_ethernet(data)?;
    if ethernet.ethertype != 0x0800 {
        return Err(ParseError::InvalidEtherType);
    }

    let ipv4_data = &data[ethernet::HEADER_LEN..];
    let ipv4 = ipv4::parse_ipv4(ipv4_data)?;
    if ipv4.flags_fragment_offset & 0x3fff != 0 {
        return Err(ParseError::UnsupportedFragment);
    }
    if ipv4.protocol != udp::PROTOCOL_NUMBER {
        return Err(ParseError::UnsupportedProtocol);
    }

    let ipv4_header_len = usize::from(ipv4.ihl) * 4;
    let ipv4_total_len = usize::from(ipv4.total_length);
    let udp_data = &ipv4_data[ipv4_header_len..ipv4_total_len];
    let udp = udp::parse_udp(udp_data)?;

    Ok(Packet {
        ethernet,
        ipv4,
        udp,
    })
}
