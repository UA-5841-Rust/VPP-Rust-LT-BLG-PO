#ifndef NETWORK_PARSER_H
#define NETWORK_PARSER_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

// Must match #[repr(C)] struct in Rust exactly
typedef struct {
    bool is_valid;
    uint8_t protocol;
    uint16_t dest_port;
    uint32_t error_code;
} ClassifyResult;

// FFI function signature
ClassifyResult packet_classify(const uint8_t* data, size_t len);

#endif // NETWORK_PARSER_H