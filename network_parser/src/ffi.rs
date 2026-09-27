use crate::Packet;
use crate::error::ParseError;
use crate::parse_packet;
use std::slice;

pub struct PacketHandle<'a>(pub Packet<'a>);

// SAFETY: The VPP C plugin guarantees that `data` is a valid pointer to a packet buffer
// and `len` accurately reflects the buffer's current length. The slice is only used
// temporarily during the classification step and does not outlive this function call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn packet_parse(data: *const u8, len: usize) -> *mut PacketHandle<'static> {
    if data.is_null() || len == 0 {
        return std::ptr::null_mut();
    }

    let slice = unsafe { slice::from_raw_parts(data, len) };

    match parse_packet(slice) {
        Ok(packet) => {
            let handle = Box::new(PacketHandle(packet));

            unsafe {
                let handle_static: Box<PacketHandle<'static>> = std::mem::transmute(handle);
                Box::into_raw(handle_static)
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

// SAFETY: The VPP C plugin guarantees that `data` is a valid pointer to a packet buffer
// and `len` accurately reflects the buffer's current length. The slice is only used
// temporarily during the classification step and does not outlive this function call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn packet_free(handle: *mut PacketHandle<'static>) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle) };
    }
}

#[repr(C)]
pub struct ClassifyResult {
    pub is_valid: bool,
    pub protocol: u8,
    pub dest_port: u16,
    pub error_code: u32,
}

// SAFETY: The VPP C plugin guarantees that `data` is a valid pointer to a packet buffer
// and `len` accurately reflects the buffer's current length. The slice is only used
// temporarily during the classification step and does not outlive this function call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn packet_classify(data: *const u8, len: usize) -> ClassifyResult {
    if data.is_null() || len == 0 {
        return ClassifyResult {
            is_valid: false,
            protocol: 0,
            dest_port: 0,
            error_code: ParseError::PacketTooShort as u32,
        };
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };

    match parse_packet(slice) {
        Ok(packet) => {
            if let Some(udp) = packet.udp {
                ClassifyResult {
                    is_valid: true,
                    protocol: 1,
                    dest_port: udp.destination_port,
                    error_code: 0,
                }
            } else {
                ClassifyResult {
                    is_valid: true,
                    protocol: 0,
                    dest_port: 0,
                    error_code: 0,
                }
            }
        }
        Err(e) => ClassifyResult {
            is_valid: false,
            protocol: 0,
            dest_port: 0,
            error_code: e as u32,
        },
    }
}
