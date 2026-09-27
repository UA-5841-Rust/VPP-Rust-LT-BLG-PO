use network_parser::error::ParseError;
use network_parser::ipv4::Ipv4Header;

#[test]
fn test_ipv4_valid_with_options() {
    let mut data = vec![0u8; 24];
    data[0] = 0x46;
    data[2] = 0x00;
    data[3] = 24;

    let result = Ipv4Header::parse(&data);
    assert!(result.is_ok());
}

#[test]
fn test_ipv4_invalid_version() {
    let mut data = vec![0u8; 20];
    data[0] = 0x55;

    let result = Ipv4Header::parse(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4Version));
}

#[test]
fn test_ipv4_invalid_ihl() {
    let mut data = vec![0u8; 20];
    data[0] = 0x44;

    let result = Ipv4Header::parse(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4HeaderLength));
}

#[test]
fn test_ipv4_invalid_total_length() {
    let mut data = vec![0u8; 20];
    data[0] = 0x45;
    data[2] = 0x00;
    data[3] = 10;

    let result = Ipv4Header::parse(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4TotalLength));
}

#[test]
fn test_ipv4_truncated_packet() {
    let mut data = vec![0u8; 25];
    data[0] = 0x45;
    data[2] = 0x00;
    data[3] = 40;

    let result = Ipv4Header::parse(&data);
    assert_eq!(result, Err(ParseError::PacketTooShort));
}
