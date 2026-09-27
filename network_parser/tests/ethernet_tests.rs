use network_parser::error::ParseError;
use network_parser::ethernet::EthernetHeader;

#[test]
fn test_ethernet_valid() {
    let mut data = vec![0u8; 14];
    data[12] = 0x08; // EtherType = IPv4
    data[13] = 0x00;

    let result = EthernetHeader::parse(&data);
    assert!(result.is_ok());
}

#[test]
fn test_ethernet_too_short() {
    let data = vec![0u8; 10]; // < min 14 bytes
    let result = EthernetHeader::parse(&data);

    assert_eq!(result, Err(ParseError::PacketTooShort));
}

#[test]
fn test_ethernet_invalid_ethertype() {
    let mut data = vec![0u8; 14];
    data[12] = 0x86; // IPv6 instead of IPv4
    data[13] = 0xDD;

    let result = EthernetHeader::parse(&data);

    assert_eq!(result, Err(ParseError::InvalidEtherType));
}
