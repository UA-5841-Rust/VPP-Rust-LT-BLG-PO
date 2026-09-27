use network_parser::error::ParseError;
use network_parser::udp::UdpPacket;

#[test]
fn test_udp_empty_payload() {
    let mut data = vec![0u8; 8];
    data[4] = 0x00;
    data[5] = 8;

    let result = UdpPacket::parse(&data);
    assert!(result.is_ok());

    let packet = result.unwrap();
    assert_eq!(packet.payload.len(), 0);
}

#[test]
fn test_udp_invalid_length() {
    let mut data = vec![0u8; 8];
    data[4] = 0x00;
    data[5] = 6;

    let result = UdpPacket::parse(&data);
    assert_eq!(result, Err(ParseError::InvalidUdpLength));
}

#[test]
fn test_udp_truncated_payload() {
    let mut data = vec![0u8; 10];
    data[4] = 0x00;
    data[5] = 12;

    let result = UdpPacket::parse(&data);
    assert_eq!(result, Err(ParseError::PacketTooShort));
}
