use network_parser::ParseError;
use network_parser::ipv4::parse_ipv4_header;

fn valid_ipv4_bytes(ihl: u8, options: Option<&[u8]>) -> Vec<u8> {
    let mut packet = vec![];
    let header_len = (ihl as usize) * 4;

    // Version (4) | IHL
    packet.push((4 << 4) | ihl);

    // DSCP + ECN
    packet.push(0x00);

    // Total Length = header only, for simplicity
    packet.extend_from_slice(&(header_len as u16).to_be_bytes());

    // Identification
    packet.extend_from_slice(&[0x12, 0x34]);

    // Flags + Fragment Offset (not fragmented)
    packet.extend_from_slice(&[0x00, 0x00]);

    // TTL
    packet.push(64);

    // Protocol = UDP
    packet.push(17);

    // Checksum
    packet.extend_from_slice(&[0x00, 0x00]);

    // Source IP
    packet.extend_from_slice(&[192, 168, 1, 1]);

    // Destination IP
    packet.extend_from_slice(&[192, 168, 1, 2]);

    if let Some(opts) = options {
        // Options (length not validated against IHL)
        packet.extend_from_slice(opts);
    }
    packet
}

#[test]
fn test_valid_ipv4_no_options() {
    let data = valid_ipv4_bytes(5, None);
    let (ip, len) = parse_ipv4_header(&data).unwrap();
    assert_eq!(ip.version, 4);
    assert_eq!(ip.ihl, 5);
    assert_eq!(len, 20);
    assert_eq!(ip.src_addr, [192, 168, 1, 1]);
    assert_eq!(ip.dest_addr, [192, 168, 1, 2]);
    assert_eq!(ip.protocol, 17);
}

#[test]
fn test_ipv4_with_options() {
    let options = &[0x01, 0x02, 0x03, 0x04];
    let data = valid_ipv4_bytes(6, Some(options));
    let (ip, len) = parse_ipv4_header(&data).unwrap();
    assert_eq!(ip.ihl, 6);
    assert_eq!(len, 24);
}

#[test]
fn test_ipv4_too_short() {
    let data = vec![0x45; 10];
    let result = parse_ipv4_header(&data);
    assert_eq!(result, Err(ParseError::PacketTooShort));
}

#[test]
fn test_truncated_ipv4_header() {
    // IHL=6 promises 24 bytes, but the slice is only 22.
    let mut data = valid_ipv4_bytes(6, Some(&[0x01, 0x02, 0x03, 0x04]));
    data.truncate(22);

    let result = parse_ipv4_header(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4HeaderLength));
}

#[test]
fn test_invalid_version() {
    let mut data = valid_ipv4_bytes(5, None);
    // Top 4 bits = 6 -> IPv6
    data[0] = 0x65;
    let result = parse_ipv4_header(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4Version));
}

#[test]
fn test_invalid_ihl() {
    let mut data = valid_ipv4_bytes(5, None);
    // IHL = 4 (< 5) means header < 20 bytes, which is invalid.
    data[0] = 0x44;
    let result = parse_ipv4_header(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4HeaderLength));
}

#[test]
fn test_invalid_total_length() {
    let mut data = valid_ipv4_bytes(5, None);
    // Total Length = 5, less than header_len (20) -> invalid.
    data[2] = 0x00;
    data[3] = 0x05;
    let result = parse_ipv4_header(&data);
    assert_eq!(result, Err(ParseError::InvalidIpv4TotalLength));
}

#[test]
fn test_unsupported_protocol() {
    let mut data = valid_ipv4_bytes(5, None);
    // TCP instead of UDP
    data[9] = 6;
    let result = parse_ipv4_header(&data);
    assert_eq!(result, Err(ParseError::UnsupportedProtocol));
}
