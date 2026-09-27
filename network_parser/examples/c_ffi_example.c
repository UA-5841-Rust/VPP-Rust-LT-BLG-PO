#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

// 1. Forward declaration of the "opaque" struct from Rust
typedef struct PacketHandle PacketHandle;

// 2. Declare the signatures of the functions we wrote in ffi.rs
extern PacketHandle* packet_parse(const uint8_t* data, size_t len);
extern void packet_free(PacketHandle* handle);

int main() {
    printf("Starting C FFI example...\n");

    // 3. Create a mock buffer (minimum 46 bytes for Eth + IPv4 + UDP)
    size_t packet_len = 64;
    uint8_t* raw_packet = (uint8_t*)calloc(packet_len, sizeof(uint8_t));

    if (raw_packet == NULL) {
        printf("Memory allocation failed!\n");
        return 1;
    }

    // 4. Call the Rust code! Pass the raw pointer and length
    PacketHandle* handle = packet_parse(raw_packet, packet_len);

    if (handle != NULL) {
        printf("SUCCESS: Rust parsed the packet and returned a handle!\n");

        // 5. Always free the memory through Rust to prevent memory leaks
        packet_free(handle);
        printf("SUCCESS: Memory successfully freed by Rust.\n");
    } else {
        printf("INFO: Rust parser correctly rejected the invalid/empty packet.\n");
    }

    free(raw_packet);
    return 0;
}