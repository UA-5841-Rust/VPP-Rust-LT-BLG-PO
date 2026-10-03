#include "network_parser.h"
#include <assert.h>
#include <stdio.h>

int main(void)
{
  uint8_t packet[42] = {0};
  packet[12] = 8;
  packet[14] = 0x45;
  packet[17] = 28;
  packet[23] = 17;
  packet[36] = 0x10;
  packet[37] = 0xe1; /* 4321 */
  packet[39] = 8;
  /* SAFETY: stack array is initialized, live, contiguous and unchanged for
   * each call; all non-sentinel lengths below fit the allocation. */
  ClassifyResult r = packet_classify(packet, sizeof packet);
  assert(r.is_valid && r.protocol == 1 && r.dest_port == 4321 && r.error_code == 0);
  for (size_t len = 0; len < sizeof packet; ++len)
    assert(!packet_classify(packet, len).is_valid);
  /* These sentinel inputs are rejected before dereferencing in Rust. */
  assert(packet_classify(NULL, 0).error_code == PACKET_TOO_SHORT);
  assert(packet_classify(NULL, 1).error_code == PACKET_NULL_POINTER);
  assert(packet_classify(packet, SIZE_MAX).error_code == PACKET_INVALID_LENGTH);
  puts("C/Rust ABI smoke test passed");
  return 0;
}
