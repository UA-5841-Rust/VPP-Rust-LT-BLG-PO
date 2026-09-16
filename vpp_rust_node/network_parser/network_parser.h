#ifndef NETWORK_PARSER_H
#define NETWORK_PARSER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Matches #[repr(C)] ClassifyResult in Rust */
typedef struct {
    bool is_valid;
    uint8_t protocol;
    uint16_t dest_port;
    uint32_t error_code;
} ClassifyResult;

/* FFI interface to the Rust parser */
ClassifyResult packet_classify(const uint8_t* data, size_t len);

#endif /* NETWORK_PARSER_H */