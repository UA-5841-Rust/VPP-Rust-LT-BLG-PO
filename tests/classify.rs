use network_parser::ffi::{classify, packet_classify, ClassifyError, ClassifyResult};

fn packet() -> [u8; 46] {
    let mut bytes = [0; 46];
    bytes[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    bytes[14] = 0x45;
    bytes[16..18].copy_from_slice(&32u16.to_be_bytes());
    bytes[23] = 17;
    bytes[36..38].copy_from_slice(&4321u16.to_be_bytes());
    bytes[38..40].copy_from_slice(&12u16.to_be_bytes());
    bytes
}

fn call(bytes: &[u8]) -> ClassifyResult {
    // SAFETY: slice provides its actual length and one live readable allocation;
    // shared borrow prevents mutation during the call. Empty slices are allowed.
    unsafe { packet_classify(bytes.as_ptr(), bytes.len()) }
}

#[test]
fn returns_udp_fields_and_matches_safe_parser() {
    let bytes = packet();
    assert_eq!(
        call(&bytes),
        ClassifyResult {
            is_valid: true,
            protocol: 1,
            dest_port: 4321,
            error_code: 0,
        }
    );
    assert_eq!(call(&bytes), classify(&bytes));
}

#[test]
fn every_truncated_prefix_is_rejected() {
    let bytes = packet();
    for end in 0..bytes.len() {
        let result = call(&bytes[..end]);
        assert!(!result.is_valid, "prefix {end}");
        assert_ne!(result.error_code, 0);
        assert_eq!((result.protocol, result.dest_port), (0, 0));
    }
}

#[test]
fn error_mapping_is_stable() {
    for (offset, value, expected) in [
        (12, 0x86, ClassifyError::InvalidEtherType),
        (14, 0x65, ClassifyError::InvalidIpv4Version),
        (14, 0x44, ClassifyError::InvalidIpv4HeaderLength),
        (17, 19, ClassifyError::InvalidIpv4TotalLength),
        (39, 7, ClassifyError::InvalidUdpLength),
        (23, 6, ClassifyError::UnsupportedProtocol),
        (20, 0x20, ClassifyError::UnsupportedFragment),
        (21, 1, ClassifyError::UnsupportedFragment),
    ] {
        let mut bytes = packet();
        bytes[offset] = value;
        assert_eq!(call(&bytes).error_code, expected as u32);
    }
}

#[test]
fn checks_ffi_sentinels_before_constructing_a_slice() {
    // SAFETY: null pointers are explicitly accepted and rejected before access.
    unsafe {
        assert_eq!(packet_classify(std::ptr::null(), 0).error_code, 2);
        assert_eq!(packet_classify(std::ptr::null(), 1).error_code, 1);
    }
    let byte = 0u8;
    // SAFETY: oversized lengths are explicitly rejected before dereference;
    // this does not ask Rust to construct a slice larger than the allocation.
    let result = unsafe { packet_classify(&byte, isize::MAX as usize + 1) };
    assert_eq!(result.error_code, ClassifyError::InvalidLength as u32);
}

#[test]
fn ethernet_padding_is_ignored_and_df_is_allowed() {
    let mut bytes = packet().to_vec();
    bytes[20] = 0x40;
    bytes.extend_from_slice(&[0; 18]);
    assert!(call(&bytes).is_valid);
}

#[test]
fn result_layout_matches_c_header() {
    assert_eq!(std::mem::size_of::<ClassifyResult>(), 8);
    assert_eq!(std::mem::align_of::<ClassifyResult>(), 4);
    assert_eq!(std::mem::offset_of!(ClassifyResult, protocol), 1);
    assert_eq!(std::mem::offset_of!(ClassifyResult, dest_port), 2);
    assert_eq!(std::mem::offset_of!(ClassifyResult, error_code), 4);
}

#[test]
fn arbitrary_bytes_never_panic_and_ffi_matches_safe_entry() {
    let mut bytes = [0u8; 256];
    let mut seed = 0x12345678u32;
    for round in 0..4096 {
        for byte in &mut bytes {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *byte = seed as u8;
        }
        let input = &bytes[..round % 257];
        assert_eq!(call(input), classify(input));
    }
}
