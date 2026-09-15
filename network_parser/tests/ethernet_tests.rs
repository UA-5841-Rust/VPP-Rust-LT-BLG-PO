use network_parser::ParseError;
use network_parser::ethernet::{EthernetType, parse_ethernet_header};

fn valid_ethernet_bytes() -> Vec<u8> {
    let mut packet = vec![];

    // Destination MAC
    packet.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

    // Source MAC
    packet.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);

    // Ethernet type = IPv4
    packet.extend_from_slice(&[0x08, 0x00]);

    packet
}

#[test]
fn test_valid_ethernet() {
    let data = valid_ethernet_bytes();
    let eth = parse_ethernet_header(&data).unwrap();
    assert_eq!(eth.dest_mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    assert_eq!(eth.src_mac, [0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);
    assert!(matches!(eth.ether_type, EthernetType::IpV4));
}

#[test]
fn test_ethernet_too_short() {
    let data = vec![0x00; 5];
    let result = parse_ethernet_header(&data);
    assert_eq!(result, Err(ParseError::PacketTooShort));
}

#[test]
fn test_invalid_ether_type() {
    let mut data = valid_ethernet_bytes();
    // ARP instead of IPv4
    data[12] = 0x08;
    data[13] = 0x06;
    let result = parse_ethernet_header(&data);
    assert_eq!(result, Err(ParseError::InvalidEtherType));
}
