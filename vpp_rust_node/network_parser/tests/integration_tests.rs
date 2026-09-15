use network_parser::{ParseError, parse_packet};

fn build_test_packet() -> Vec<u8> {
    vec![
        // --- ETHERNET II (14 bytes) ---
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, // Destination MAC
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, // Source MAC
        0x08, 0x00, // EtherType (IPv4)
        // --- IPv4 (20 bytes) ---
        0x45, // Version (4) + IHL (5 words = 20 bytes)
        0x00, // DSCP/ECN
        0x00, 0x21, // Total Length: 33 bytes (20 IP + 8 UDP + 5 Payload)
        0x00, 0x00, // Identification
        0x00, 0x00, // Flags / Fragment Offset
        0x40, // TTL (64)
        0x11, // Protocol (UDP = 17 = 0x11)
        0x00, 0x00, // Header Checksum
        0x7F, 0x00, 0x00, 0x01, // Source IP (127.0.0.1)
        0x7F, 0x00, 0x00, 0x01, // Destination IP (127.0.0.1)
        // --- UDP (8 bytes) ---
        0x04, 0xD2, // Source Port (1234)
        0x16, 0x2E, // Destination Port (5678)
        0x00, 0x0D, // Length: 13 bytes (8 header + 5 Payload)
        0x00, 0x00, // Checksum
        // --- PAYLOAD (5 bytes) ---
        b'h', b'e', b'l', b'l', b'o', // "hello"
    ]
}

#[test]
fn test_successful_parsing() {
    let data = build_test_packet();

    let packet = parse_packet(&data).expect("Parsing failed due to error");
    let ipv4 = packet.ipv4.expect("Missing IPv4 header");

    assert_eq!(ipv4.version, 4);
    assert_eq!(ipv4.protocol, 17);

    let udp = packet.udp.expect("Missing UDP header");
    assert_eq!(udp.source_port, 1234);
    assert_eq!(udp.length, 13);
    assert_eq!(udp.payload, b"hello");
}

#[test]
fn test_zero_copy_proof() {
    let data = build_test_packet();
    let packet = parse_packet(&data).unwrap();
    let udp = packet.udp.unwrap();

    let data_ptr = data.as_ptr() as usize;
    let payload_ptr = udp.payload.as_ptr() as usize;

    let expected_payload_address = data_ptr + 42;

    assert_eq!(
        payload_ptr, expected_payload_address,
        "The Zero-Copy proof has failed: the payload points to a different memory location!"
    )
}

#[test]
fn test_packet_too_short() {
    let data = build_test_packet();

    let truncated_data = &data[0..10];
    let result = parse_packet(truncated_data);
    assert_eq!(result.unwrap_err(), ParseError::PacketTooShort);
}

#[test]
fn test_invalid_ethertype() {
    let mut data = build_test_packet();
    data[12] = 0x86;
    data[13] = 0xDD;

    let result = parse_packet(&data);
    assert_eq!(result.unwrap_err(), ParseError::InvalidEtherType);
}

#[test]
fn test_invalid_ipv4_version() {
    let mut data = build_test_packet();
    data[14] = 0x55;

    let result = parse_packet(&data);
    assert_eq!(result.unwrap_err(), ParseError::InvalidIpv4Version);
}

#[test]
fn test_invalid_ipv4_header_length() {
    let mut data = build_test_packet();
    data[14] = 0x44;

    let result = parse_packet(&data);
    assert_eq!(result.unwrap_err(), ParseError::InvalidIpv4HeaderLength);
}

#[test]
fn test_invalid_ipv4_total_length() {
    let mut data = build_test_packet();

    data[16] = 0x00;
    data[17] = 0x0A; // 10

    let result = parse_packet(&data);
    assert_eq!(result.unwrap_err(), ParseError::InvalidIpv4TotalLength);
}

#[test]
fn test_invalid_udp_length() {
    let mut data = build_test_packet();
    data[38] = 0x00;
    data[39] = 0x04; // 4

    let result = parse_packet(&data);
    assert_eq!(result.unwrap_err(), ParseError::InvalidUdpLength);
}

#[test]
fn test_truncated_payload() {
    let mut data = build_test_packet();

    data.pop();
    data.pop();

    let result = parse_packet(&data);
    assert_eq!(result.unwrap_err(), ParseError::PacketTooShort);
}

#[test]
fn test_empty_udp_payload() {
    let mut data = build_test_packet();

    // IPv4 Total Length (20 IP + 8 UDP = 28)
    data[16] = 0x00;
    data[17] = 28;

    data[38] = 0x00;
    data[39] = 8;

    data.truncate(42);

    let packet = parse_packet(&data).unwrap();
    let udp = packet.udp.unwrap();

    assert_eq!(udp.length, 8);
    assert_eq!(udp.payload.len(), 0);
}

#[test]
fn test_ipv4_with_options() {
    let mut data = build_test_packet();

    data[14] = 0x46;

    data.insert(34, 0x00);
    data.insert(34, 0x00);
    data.insert(34, 0x00);
    data.insert(34, 0x00);

    data[16] = 0x00;
    data[17] = 37;

    let packet = parse_packet(&data).unwrap();
    let ipv4 = packet.ipv4.unwrap();

    assert_eq!(ipv4.ihl, 6);
    let udp = packet.udp.unwrap();
    assert_eq!(udp.source_port, 1234);
    assert_eq!(udp.payload, b"hello");
}
