#ifndef NETWORK_PARSER_H
#define NETWORK_PARSER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

enum packet_classify_error {
  PACKET_OK = 0,
  PACKET_NULL_POINTER = 1,
  PACKET_TOO_SHORT = 2,
  PACKET_INVALID_ETHERTYPE = 3,
  PACKET_INVALID_IPV4_VERSION = 4,
  PACKET_INVALID_IPV4_HEADER_LENGTH = 5,
  PACKET_INVALID_IPV4_TOTAL_LENGTH = 6,
  PACKET_INVALID_UDP_LENGTH = 7,
  PACKET_UNSUPPORTED_PROTOCOL = 8,
  PACKET_MALFORMED = 9,
  PACKET_INVALID_LENGTH = 10,
  PACKET_UNSUPPORTED_FRAGMENT = 11
};

typedef struct {
  bool is_valid;
  uint8_t protocol; /* 0 unknown, 1 UDP; NOT the IP protocol number */
  uint16_t dest_port; /* host byte order; zero on failure */
  uint32_t error_code;
} ClassifyResult;

_Static_assert(sizeof(ClassifyResult) == 8, "Rust/C result size mismatch");
_Static_assert(offsetof(ClassifyResult, protocol) == 1, "protocol ABI mismatch");
_Static_assert(offsetof(ClassifyResult, dest_port) == 2, "port ABI mismatch");
_Static_assert(offsetof(ClassifyResult, error_code) == 4, "error ABI mismatch");

/* Caller owns one live, initialized, contiguous region of len bytes beginning
 * at Ethernet II. Keep it readable and unmodified until return. No pointer is
 * retained. Null and zero lengths return errors without dereference. An invalid
 * non-null pointer or a falsely claimed allocation length is a caller bug. */
ClassifyResult packet_classify(const uint8_t *data, size_t len);

#endif
