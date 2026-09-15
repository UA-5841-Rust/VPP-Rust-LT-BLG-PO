#ifndef __included_rust_classify_h__
#define __included_rust_classify_h__

#include <vnet/vnet.h>
#include <vnet/ip/ip.h>
#include <vnet/ethernet/ethernet.h>
#include <vppinfra/error.h>
#include <vppinfra/hash.h>

typedef struct
{
	/* convenience: pointer back to the shared vnet_main_t */
	vnet_main_t *vnet_main;
} rust_classify_main_t;

extern rust_classify_main_t rust_classify_main;
extern vlib_node_registration_t rust_classify_node;

/* Volatile flag toggled by the CLI to skip FFI calls.
 * Used in Part B to isolate the cost of the Rust packet_classify call. */
extern volatile u8 rust_classify_passthrough;

#endif /* __included_rust_classify_h__ */
