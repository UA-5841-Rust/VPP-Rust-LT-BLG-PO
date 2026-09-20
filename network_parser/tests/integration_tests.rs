use network_parser::ParseError;
use network_parser::parse_packet;

fn valid_full_packet(payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![];

    // Destination MAC
    packet.extend_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

    // Source MAC
    packet.extend_from_slice(&[0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB]);

    // Ether type = 0x0800 (IPv4)
    packet.extend_from_slice(&[0x08, 0x00]);

    let udp_len = (8 + payload.len()) as u16;
    let ipv4_header_len: u16 = 20;
    let total_len = ipv4_header_len + udp_len;

    // Version (4) | IHL (5, i.e. 20-byte header, no options)
    packet.push((4 << 4) | 5);

    // DSCP + ECN
    packet.push(0x00);

    // Total Length = IPv4 header (20) + UDP header/payload
    packet.extend_from_slice(&total_len.to_be_bytes());

    // Identification
    packet.extend_from_slice(&[0x12, 0x34]);

    // Flags + Fragment Offset (not fragmented)
    packet.extend_from_slice(&[0x00, 0x00]);

    // TTL = 64
    packet.push(64);

    // Protocol = 17 (UDP)
    packet.push(17);

    // Checksum
    packet.extend_from_slice(&[0x00, 0x00]);

    // Source IP = 192.168.1.1
    packet.extend_from_slice(&[192, 168, 1, 1]);

    // Destination IP = 192.168.1.2
    packet.extend_from_slice(&[192, 168, 1, 2]);

    // Source port = 0x0050 = 80
    packet.extend_from_slice(&[0x00, 0x50]);

    // Destination port = 0x1F90 = 8080
    packet.extend_from_slice(&[0x1F, 0x90]);

    // UDP Length = header (8) + payload
    packet.extend_from_slice(&udp_len.to_be_bytes());

    // Checksum
    packet.extend_from_slice(&[0x00, 0x00]);

    packet.extend_from_slice(payload);
    packet
}

#[test]
fn test_valid_full_packet() {
    let payload = &[0xDE, 0xAD, 0xBE, 0xEF];
    let data = valid_full_packet(payload);
    let packet = parse_packet(&data).unwrap();

    assert_eq!(
        packet.ethernet.dest_mac,
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );
    assert_eq!(packet.ipv4.src_addr, [192, 168, 1, 1]);
    assert_eq!(packet.ipv4.dest_addr, [192, 168, 1, 2]);
    assert_eq!(packet.udp.src_port, 80);
    assert_eq!(packet.udp.dest_port, 8080);
    assert_eq!(packet.udp.payload, payload);
}

#[test]
fn test_packet_too_short_for_ethernet() {
    let data = vec![0x00; 10];
    let result = parse_packet(&data);
    assert_eq!(result, Err(ParseError::PacketTooShort));
}

#[test]
fn test_invalid_ether_type_stops_parsing() {
    let mut data = valid_full_packet(&[0xDE, 0xAD]);
    // ARP (0x0806) instead of IPv4 (0x0800), should fail before touching IPv4/UDP
    data[12] = 0x08;
    data[13] = 0x06;
    let result = parse_packet(&data);
    assert_eq!(result, Err(ParseError::InvalidEtherType));
}

#[test]
fn test_invalid_ipv4_version_in_full_packet() {
    let mut data = valid_full_packet(&[0xDE, 0xAD]);
    // IPv4 header starts right after the 14-byte Ethernet header;
    // top 4 bits = 6 -> IPv6
    data[14] = 0x65;
    let result = parse_packet(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4Version));
}

#[test]
fn test_invalid_udp_length_in_full_packet() {
    let mut data = valid_full_packet(&[0xDE, 0xAD, 0xBE, 0xEF]);
    // UDP header starts at 14 (Ethernet) + 20 (IPv4) = 34.
    // Length field = 100, greater than remaining buffer -> invalid
    data[34 + 4] = 0x00;
    data[34 + 5] = 0x64;
    let result = parse_packet(&data);
    assert_eq!(result, Err(ParseError::InvalidUdpLength));
}

#[test]
fn test_empty_udp_payload() {
    let data = valid_full_packet(&[]);
    let packet = parse_packet(&data).unwrap();
    assert_eq!(packet.udp.len, 8);
    assert_eq!(packet.udp.payload, &[]);
}

#[test]
fn test_truncated_packet() {
    let mut data = valid_full_packet(&[0x01; 100]);
    // Truncate mid-payload: only 6 bytes remain for the UDP header (needs 8).
    data.truncate(40);
    let result = parse_packet(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4TotalLength));
}

#[test]
fn test_udp_header_too_short() {
    let mut data = valid_full_packet(&[]);

    let bad_total_len: u16 = 26;
    data[16..18].copy_from_slice(&bad_total_len.to_be_bytes());

    data.truncate(14 + 26);

    let result = parse_packet(&data);

    assert_eq!(result, Err(ParseError::PacketTooShort));
}
