/// Errors that can occur while parsing a packet.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Buffer is too short for the header being parsed.
    PacketTooShort,
    /// EtherType is not 0x0800 (IPv4).
    InvalidEtherType,
    /// IP version field is not 4.
    InvalidIpv4Version,
    /// IHL claims a header size that's invalid (< 20 bytes) or doesn't fit the buffer.
    InvalidIpv4HeaderLength,
    /// Total Length field is smaller than the header itself.
    InvalidIpv4TotalLength,
    /// UDP Length field is smaller than the header or larger than the buffer.
    InvalidUdpLength,
    /// IPv4 protocol field is not 17 (UDP) — only UDP is supported.
    UnsupportedProtocol,
}
