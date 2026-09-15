//! Zero-copy network packet parser.
//!
//! Parses Ethernet II → IPv4 → UDP without copying the payload.
//! Exposes a safe Rust API and a C-compatible FFI layer.

pub mod error;
pub mod ethernet;
pub mod ffi;
pub mod ipv4;
pub mod udp;

pub use error::ParseError;
pub use ethernet::EthernetHeader;
pub use ipv4::IpV4Header;
pub use udp::UdpPacket;

/// Fully parsed network packet.
///
/// All header fields are owned copies. The UDP payload is a zero-copy slice
/// referencing the original input buffer.
#[derive(Debug, PartialEq, Eq)]
pub struct Packet<'a> {
    pub ethernet: EthernetHeader,
    pub ipv4: IpV4Header,
    pub udp: UdpPacket<'a>,
}

/// Parses a raw Ethernet/IPv4/UDP packet without copying the payload.
///
/// Runs the three parsers in order (Ethernet -> IPv4 -> UDP) and bails
/// out on the first error. IPv4 options are skipped automatically since
/// we use the actual header length, not the fixed 20-byte minimum.
pub fn parse_packet(data: &[u8]) -> Result<Packet<'_>, ParseError> {
    let ethernet = ethernet::parse_ethernet_header(data)?;
    let (ipv4, header_len) = ipv4::parse_ipv4_header(&data[EthernetHeader::SIZE..])?;

    let ip_end_idx = EthernetHeader::SIZE + (ipv4.total_len as usize);
    if data.len() < ip_end_idx {
        return Err(ParseError::InvalidIpv4TotalLength);
    }

    let udp_start_idx = EthernetHeader::SIZE + header_len;
    let udp = udp::parse_udp_packet(&data[udp_start_idx..ip_end_idx])?;

    Ok(Packet {
        ethernet,
        ipv4,
        udp,
    })
}
