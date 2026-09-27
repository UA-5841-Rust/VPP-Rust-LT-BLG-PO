use network_parser::ffi::packet_classify;

fn build_valid_udp_packet() -> Vec<u8> {
    let mut p = vec![0u8; 46];
    p[12] = 0x08;
    p[13] = 0x00;
    p[14] = 0x45;
    p[16] = 0x00;
    p[17] = 32;
    p[23] = 17;
    p[26] = 192;
    p[27] = 168;
    p[28] = 0;
    p[29] = 1;
    p[30] = 192;
    p[31] = 168;
    p[32] = 0;
    p[33] = 1;
    p[34] = 0x04;
    p[35] = 0xD2;
    p[36] = 0x16;
    p[37] = 0x2E;
    p[38] = 0x00;
    p[39] = 12;
    p[42] = 0xAA;
    p[43] = 0xBB;
    p[44] = 0xCC;
    p[45] = 0xDD;
    p
}

#[test]
fn classify_null_ptr() {
    let r = unsafe { packet_classify(std::ptr::null(), 64) };
    assert!(!r.is_valid);
    assert_eq!(r.error_code, 1);
}

#[test]
fn classify_zero_len() {
    let data = [0u8; 4];
    let r = unsafe { packet_classify(data.as_ptr(), 0) };
    assert!(!r.is_valid);
    assert_eq!(r.error_code, 1);
}

#[test]
fn classify_too_short() {
    let data = [0u8; 10];
    let r = unsafe { packet_classify(data.as_ptr(), data.len()) };
    assert!(!r.is_valid);
    assert_eq!(r.error_code, 1);
}

#[test]
fn classify_valid_udp() {
    let p = build_valid_udp_packet();
    let r = unsafe { packet_classify(p.as_ptr(), p.len()) };
    assert!(r.is_valid);
    assert_eq!(r.protocol, 1);
    assert_eq!(r.dest_port, 5678);
    assert_eq!(r.error_code, 0);
}

#[test]
fn classify_ipv6_ethertype_unsupported() {
    let mut p = build_valid_udp_packet();
    p[12] = 0x86;
    p[13] = 0xDD;
    let r = unsafe { packet_classify(p.as_ptr(), p.len()) };
    assert!(!r.is_valid);
    assert_eq!(r.error_code, 2);
}

#[test]
fn classify_tcp_unsupported() {
    let mut p = build_valid_udp_packet();
    p[23] = 6;
    let r = unsafe { packet_classify(p.as_ptr(), p.len()) };
    assert!(!r.is_valid);
    assert_eq!(r.error_code, 7);
}

#[test]
fn classify_ipv4_with_options_valid() {
    let mut p = vec![0u8; 50];
    p[12] = 0x08;
    p[13] = 0x00;
    p[14] = 0x46;
    p[16] = 0x00;
    p[17] = 36;
    p[23] = 17;
    p[26] = 192;
    p[27] = 168;
    p[28] = 0;
    p[29] = 1;
    p[30] = 192;
    p[31] = 168;
    p[32] = 0;
    p[33] = 1;
    p[38] = 0x04;
    p[39] = 0xD2;
    p[40] = 0x16;
    p[41] = 0x2E;
    p[42] = 0x00;
    p[43] = 12;
    p[46] = 0xAA;
    p[47] = 0xBB;
    p[48] = 0xCC;
    p[49] = 0xDD;
    let r = unsafe { packet_classify(p.as_ptr(), p.len()) };
    assert!(r.is_valid);
    assert_eq!(r.dest_port, 5678);
}

#[test]
fn classify_truncated_ip() {
    let mut p = vec![0u8; 30];
    p[12] = 0x08;
    p[13] = 0x00;
    p[14] = 0x45;
    p[23] = 17;
    p[16] = 0x00;
    p[17] = 32;
    let r = unsafe { packet_classify(p.as_ptr(), p.len()) };
    assert!(!r.is_valid);
    assert_eq!(r.error_code, 1);
}
