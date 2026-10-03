/// Errors returned while parsing an Ethernet/IPv4/UDP packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// The supplied buffer ends before a required field can be read.
    PacketTooShort,
    /// The Ethernet frame does not carry IPv4 (`0x0800`).
    InvalidEtherType,
    /// The IP version is not IPv4.
    InvalidIpv4Version,
    /// The IPv4 IHL value is smaller than the minimum header length.
    InvalidIpv4HeaderLength,
    /// The IPv4 total length is inconsistent with the header or supplied buffer.
    InvalidIpv4TotalLength,
    /// The UDP length is smaller than its header or inconsistent with the IP payload.
    InvalidUdpLength,
    /// The IPv4 payload is not UDP.
    UnsupportedProtocol,
    /// IPv4 fragments require reassembly, which this parser does not perform.
    UnsupportedFragment,
    /// The packet has an invalid structure not covered by a more specific error.
    MalformedPacket,
}
