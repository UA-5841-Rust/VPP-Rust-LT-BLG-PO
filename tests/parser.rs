use network_parser::ethernet::HEADER_LEN as ETHERNET_HEADER_LEN;
use network_parser::udp::HEADER_LEN as UDP_HEADER_LEN;
use network_parser::{parse_packet, ParseError};

fn make_packet(options: &[u8], payload: &[u8]) -> Vec<u8> {
    assert!(options.len() <= 40 && options.len().is_multiple_of(4));

    let ipv4_header_len = 20 + options.len();
    let udp_len = UDP_HEADER_LEN + payload.len();
    let ipv4_total_len = ipv4_header_len + udp_len;
    let mut packet = vec![0_u8; ETHERNET_HEADER_LEN + ipv4_total_len];

    packet[..6].copy_from_slice(&[0, 1, 2, 3, 4, 5]);
    packet[6..12].copy_from_slice(&[6, 7, 8, 9, 10, 11]);
    packet[12..14].copy_from_slice(&0x0800_u16.to_be_bytes());

    let ip_offset = ETHERNET_HEADER_LEN;
    packet[ip_offset] = 0x40 | ((ipv4_header_len / 4) as u8);
    packet[ip_offset + 1] = 0b1010_0011;
    packet[ip_offset + 2..ip_offset + 4].copy_from_slice(&(ipv4_total_len as u16).to_be_bytes());
    packet[ip_offset + 4..ip_offset + 6].copy_from_slice(&0x1234_u16.to_be_bytes());
    packet[ip_offset + 6..ip_offset + 8].copy_from_slice(&0x4000_u16.to_be_bytes());
    packet[ip_offset + 8] = 64;
    packet[ip_offset + 9] = 17;
    packet[ip_offset + 10..ip_offset + 12].copy_from_slice(&0xabcd_u16.to_be_bytes());
    packet[ip_offset + 12..ip_offset + 16].copy_from_slice(&[192, 0, 2, 1]);
    packet[ip_offset + 16..ip_offset + 20].copy_from_slice(&[198, 51, 100, 2]);
    packet[ip_offset + 20..ip_offset + ipv4_header_len].copy_from_slice(options);

    let udp_offset = ip_offset + ipv4_header_len;
    packet[udp_offset..udp_offset + 2].copy_from_slice(&1234_u16.to_be_bytes());
    packet[udp_offset + 2..udp_offset + 4].copy_from_slice(&4321_u16.to_be_bytes());
    packet[udp_offset + 4..udp_offset + 6].copy_from_slice(&(udp_len as u16).to_be_bytes());
    packet[udp_offset + 6..udp_offset + 8].copy_from_slice(&0x4567_u16.to_be_bytes());
    packet[udp_offset + UDP_HEADER_LEN..].copy_from_slice(payload);

    packet
}

fn udp_offset(packet: &[u8]) -> usize {
    let ipv4_ihl_bytes = usize::from(packet[ETHERNET_HEADER_LEN] & 0x0f) * 4;
    ETHERNET_HEADER_LEN + ipv4_ihl_bytes
}

#[test]
fn parses_a_complete_ethernet_ipv4_udp_packet() {
    let packet = make_packet(&[], &[9, 8, 7, 6]);

    let parsed = parse_packet(&packet).unwrap();
    assert_eq!(parsed.ethernet.destination, [0, 1, 2, 3, 4, 5]);
    assert_eq!(parsed.ethernet.source, [6, 7, 8, 9, 10, 11]);
    assert_eq!(parsed.ethernet.ethertype, 0x0800);
    assert_eq!(parsed.ipv4.version, 4);
    assert_eq!(parsed.ipv4.ihl, 5);
    assert_eq!(parsed.ipv4.dscp_ecn, 0b1010_0011);
    assert_eq!(parsed.ipv4.source, [192, 0, 2, 1]);
    assert_eq!(parsed.ipv4.destination, [198, 51, 100, 2]);
    assert_eq!(parsed.udp.source_port, 1234);
    assert_eq!(parsed.udp.destination_port, 4321);
    assert_eq!(parsed.udp.checksum, 0x4567);
    assert_eq!(parsed.udp.payload, [9, 8, 7, 6]);
}

#[test]
fn parses_an_empty_udp_payload() {
    let packet = make_packet(&[], &[]);
    let parsed = parse_packet(&packet).unwrap();

    assert_eq!(parsed.udp.length, UDP_HEADER_LEN as u16);
    assert!(parsed.udp.payload.is_empty());
}

#[test]
fn parses_ipv4_options_without_copying() {
    let options = [1, 1, 0, 0];
    let packet = make_packet(&options, &[0xaa, 0xbb]);
    let parsed = parse_packet(&packet).unwrap();
    let parsed_options = parsed.ipv4.options.unwrap();

    assert_eq!(parsed.ipv4.ihl, 6);
    assert_eq!(parsed_options, options);
    assert_eq!(
        parsed_options.as_ptr(),
        packet[ETHERNET_HEADER_LEN + 20..].as_ptr()
    );
    assert_eq!(parsed.udp.payload, [0xaa, 0xbb]);
}

#[test]
fn udp_payload_borrows_the_original_buffer() {
    let packet = make_packet(&[], &[9, 8, 7, 6]);
    let expected_payload = udp_offset(&packet) + UDP_HEADER_LEN;

    let parsed = parse_packet(&packet).unwrap();
    assert_eq!(
        parsed.udp.payload.as_ptr(),
        packet[expected_payload..].as_ptr()
    );
}

#[test]
fn rejects_a_short_packet() {
    assert_eq!(
        parse_packet(&[0_u8; ETHERNET_HEADER_LEN - 1]).unwrap_err(),
        ParseError::PacketTooShort
    );
}

#[test]
fn rejects_an_invalid_ethertype() {
    let mut packet = make_packet(&[], &[]);
    packet[12..14].copy_from_slice(&0x86dd_u16.to_be_bytes());

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidEtherType
    );
}

#[test]
fn rejects_an_invalid_ipv4_version() {
    let mut packet = make_packet(&[], &[]);
    packet[ETHERNET_HEADER_LEN] = 0x65;

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidIpv4Version
    );
}

#[test]
fn rejects_an_invalid_ipv4_header_length() {
    let mut packet = make_packet(&[], &[]);
    packet[ETHERNET_HEADER_LEN] = 0x44;

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidIpv4HeaderLength
    );
}

#[test]
fn rejects_an_invalid_ipv4_total_length() {
    let mut packet = make_packet(&[], &[]);
    packet[ETHERNET_HEADER_LEN + 2..ETHERNET_HEADER_LEN + 4].copy_from_slice(&19_u16.to_be_bytes());

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidIpv4TotalLength
    );
}

#[test]
fn rejects_a_truncated_ipv4_packet() {
    let mut packet = make_packet(&[], &[]);
    packet[ETHERNET_HEADER_LEN + 2..ETHERNET_HEADER_LEN + 4].copy_from_slice(&40_u16.to_be_bytes());

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidIpv4TotalLength
    );
}

#[test]
fn rejects_an_invalid_udp_length() {
    let mut packet = make_packet(&[], &[]);
    let offset = udp_offset(&packet);
    packet[offset + 4..offset + 6].copy_from_slice(&7_u16.to_be_bytes());

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidUdpLength
    );
}

#[test]
fn rejects_a_truncated_udp_payload() {
    let mut packet = make_packet(&[], &[]);
    let offset = udp_offset(&packet);
    packet[offset + 4..offset + 6].copy_from_slice(&12_u16.to_be_bytes());

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::InvalidUdpLength
    );
}

#[test]
fn rejects_a_non_udp_ipv4_packet() {
    let mut packet = make_packet(&[], &[]);
    packet[ETHERNET_HEADER_LEN + 9] = 6;

    assert_eq!(
        parse_packet(&packet).unwrap_err(),
        ParseError::UnsupportedProtocol
    );
}

#[test]
fn malformed_prefixes_never_panic() {
    let packet = make_packet(&[1, 1, 0, 0], &[9, 8, 7, 6]);

    for end in 0..packet.len() {
        let _ = parse_packet(&packet[..end]);
    }
}
