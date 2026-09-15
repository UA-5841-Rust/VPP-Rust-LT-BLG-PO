use network_parser::ParseError;
use network_parser::udp::parse_udp_packet;

fn valid_udp_bytes(payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![];
    let len = (8 + payload.len()) as u16;

    // Source port - 80
    packet.extend_from_slice(&[0x00, 0x50]);

    // Destination port - 8080
    packet.extend_from_slice(&[0x1F, 0x90]);

    // Length = header + payload
    packet.extend_from_slice(&len.to_be_bytes());

    // Checksum
    packet.extend_from_slice(&[0x00, 0x00]);

    // Payload
    packet.extend_from_slice(payload);

    packet
}

#[test]
fn test_valid_udp() {
    let payload = &[0xDE, 0xAD, 0xBE, 0xEF];
    let data = valid_udp_bytes(payload);

    let udp = parse_udp_packet(&data).unwrap();
    assert_eq!(udp.src_port, 80);
    assert_eq!(udp.dest_port, 8080);
    assert_eq!(udp.len, 12); // 8 + payload len (4)
    assert_eq!(udp.payload, payload);
}

#[test]
fn test_udp_too_short() {
    let data = vec![0x00; 5];
    let result = parse_udp_packet(&data);
    assert_eq!(result, Err(ParseError::PacketTooShort));
}

#[test]
fn test_invalid_udp_length_too_small() {
    let mut data = valid_udp_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]);
    // Length = 4, less than header size (8) -> invalid.
    data[4] = 0x00;
    data[5] = 0x04;

    let result = parse_udp_packet(&data);
    assert_eq!(result, Err(ParseError::InvalidUdpLength));
}

#[test]
fn test_invalid_udp_length_too_large() {
    let mut data = valid_udp_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]);
    // Length = 100, greater than actual buffer size -> invalid.
    data[4] = 0x00;
    data[5] = 0x64;
    let result = parse_udp_packet(&data);
    assert_eq!(result, Err(ParseError::InvalidUdpLength));
}

#[test]
fn test_zero_copy_payload() {
    let payload = &[0xCA, 0xFE, 0xBA, 0xBE];
    let data = valid_udp_bytes(payload);
    let original_ptr = data.as_ptr();

    let udp = parse_udp_packet(&data).unwrap();
    let payload_ptr = udp.payload.as_ptr();

    // Payload must point inside the original buffer, not a copy.
    assert!(payload_ptr >= original_ptr);
    assert!(payload_ptr < unsafe { original_ptr.add(data.len()) });
    assert_eq!(udp.payload, payload);
}
