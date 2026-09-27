use network_parser::parse_packet;

#[test]
fn test_zero_copy_behavior() {
    let mut packet = vec![0u8; 46];

    packet[12] = 0x08;
    packet[13] = 0x00;

    packet[14] = 0x45;
    packet[16] = 0x00;
    packet[17] = 32;

    packet[38] = 0x00;
    packet[39] = 12;

    packet[42] = 0xAA;
    packet[43] = 0xBB;
    packet[44] = 0xCC;
    packet[45] = 0xDD;

    let parsed = parse_packet(&packet).expect("Failed to parse valid packet");
    let udp = parsed.udp.expect("UDP packet should be parsed");

    let original_payload_ptr = packet[42..].as_ptr();
    let parsed_payload_ptr = udp.payload.as_ptr();

    assert_eq!(
        original_payload_ptr, parsed_payload_ptr,
        "Zero-copy verification failed: the payload pointer does not match the original buffer pointer!"
    );

    assert_eq!(udp.payload, &[0xAA, 0xBB, 0xCC, 0xDD]);
}
