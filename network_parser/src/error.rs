#[derive(Debug, PartialEq)]
#[repr(u32)]
pub enum ParseError {
    PacketTooShort = 1,
    InvalidEtherType = 2,
    InvalidIpv4Version = 3,
    InvalidIpv4HeaderLength = 4,
    InvalidIpv4TotalLength = 5,
    InvalidUdpLength = 6,
    UnsupportedProtocol = 7,
}
