#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    PacketTooShort,
    InvalidEtherType,
    InvalidIpv4Version,
    InvalidIpv4HeaderLength,
    InvalidIpv4TotalLength,
    InvalidUdpLength,
    UnsupportedProtocol,
}
