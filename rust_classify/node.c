#include <vlib/vlib.h>
#include <vnet/ethernet/ethernet.h>
#include <vnet/feature/feature.h>
#include <vnet/vnet.h>
#include <vppinfra/clib.h>
#include <vppinfra/error.h>

#include <network_parser.h>
#include <rust_classify/rust_classify.h>

/* CParseError codes — mirror of `ffi.rs` enum (in rust network_parser).
 * cbindgen doesn't emit these on its own: ClassifyResult.error_code is
 * typed as a plain u32 on the Rust side (not CParseError), and cbindgen
 * only generates C types that are reachable from an extern "C"
 * function's actual signature -- so as far as it's concerned this is
 * just a number, not a type it knows to bind. This is the one place in
 * C that has to know what these numbers mean. */
#define CPARSE_OK					  0
#define CPARSE_NULL_POINTER			  1
#define CPARSE_PACKET_TOO_SHORT		  2
#define CPARSE_INVALID_ETHER_TYPE	  3
#define CPARSE_INVALID_IPV4_VERSION	  4
#define CPARSE_INVALID_IPV4_HDR_LEN	  5
#define CPARSE_INVALID_IPV4_TOTAL_LEN 6
#define CPARSE_INVALID_UDP_LENGTH	  7
#define CPARSE_UNSUPPORTED_PROTOCOL	  8

typedef struct
{
	u32 next_index;
	u32 sw_if_index;
	u8 is_valid;
	u8 protocol;
	u16 dest_port;
	u32 error_code;
} rust_classify_trace_t;

static u8 *
format_rust_classify_trace (u8 *s, va_list *args)
{
	CLIB_UNUSED (vlib_main_t * vm) = va_arg (*args, vlib_main_t *);
	CLIB_UNUSED (vlib_node_t * node) = va_arg (*args, vlib_node_t *);
	rust_classify_trace_t *t = va_arg (*args, rust_classify_trace_t *);

	s = format (s,
				"RUST-CLASSIFY: sw_if_index %u, next index %u, "
				"valid %u, protocol %u, dest_port %u, error_code %u",
				t->sw_if_index, t->next_index, t->is_valid, t->protocol, t->dest_port,
				t->error_code);
	return s;
}

vlib_node_registration_t rust_classify_node;

#define foreach_rust_classify_error                                                                \
	_ (FORWARDED_OK, "valid udp packets forwarded")                                                \
	_ (NULL_POINTER, "null pointer")                                                               \
	_ (PACKET_TOO_SHORT, "packet too short")                                                       \
	_ (INVALID_ETHER_TYPE, "invalid ethertype")                                                    \
	_ (INVALID_IPV4_VERSION, "invalid ipv4 version")                                               \
	_ (INVALID_IPV4_HDR_LEN, "invalid ipv4 header length")                                         \
	_ (INVALID_IPV4_TOTAL_LEN, "invalid ipv4 total length")                                        \
	_ (INVALID_UDP_LENGTH, "invalid udp length")                                                   \
	_ (UNSUPPORTED_PROTOCOL, "unsupported protocol")

typedef enum
{
#define _(sym, str) RUST_CLASSIFY_ERROR_##sym,
	foreach_rust_classify_error
#undef _
		RUST_CLASSIFY_N_ERROR,
} rust_classify_error_t;

static char *rust_classify_error_strings[] = {
#define _(sym, string) string,
	foreach_rust_classify_error
#undef _
};

/* The drop successor is owned by this node. The forwarding successor
 * is decided dynamically by vnet_feature_next, so we don't hardcode
 * ip4-lookup here. This ensures we don't bypass VPP's L2/L3 state
 * machine (ARP, FIB lookup, etc). */
typedef enum
{
	RUST_CLASSIFY_NEXT_DROP,
	RUST_CLASSIFY_N_NEXT,
} rust_classify_next_t;

VLIB_NODE_FN (rust_classify_node)
(vlib_main_t *vm, vlib_node_runtime_t *node, vlib_frame_t *frame)
{
	u32 n_left_from, *from, *to_next;
	rust_classify_next_t next_index;

	u32 error_counts[RUST_CLASSIFY_N_ERROR] = { 0 };

	from = vlib_frame_vector_args (frame);
	n_left_from = frame->n_vectors;
	next_index = node->cached_next_index;

	while (n_left_from > 0)
		{
			u32 n_left_to_next;

			vlib_get_next_frame (vm, node, next_index, to_next, n_left_to_next);

			while (n_left_from > 0 && n_left_to_next > 0)
				{
					u32 bi0;
					vlib_buffer_t *b0;
					u32 next0;
					u8 *data0;
					u32 len0;
					struct ClassifyResult result0;

					bi0 = from[0];
					to_next[0] = bi0;
					from += 1;
					to_next += 1;
					n_left_from -= 1;
					n_left_to_next -= 1;

					b0 = vlib_get_buffer (vm, bi0);

					if (PREDICT_FALSE (rust_classify_passthrough))
						{
							vnet_feature_next (&next0, b0);
							error_counts[RUST_CLASSIFY_ERROR_FORWARDED_OK] += 1;
						}
					else
						{
							// get pointer to current data to process
							data0 = vlib_buffer_get_current (b0);
							len0 = b0->current_length;
							result0 = packet_classify (data0, len0);

							if (result0.is_valid)
								{
									/* Let VPP handle the rest of the pipeline
									 * (ethernet-input -> ip4-lookup, etc).
									 * We do NOT advance the buffer here, because
									 * vnet_feature_next expects the buffer exactly
									 * as it was received. */
									vnet_feature_next (&next0, b0);
									error_counts[RUST_CLASSIFY_ERROR_FORWARDED_OK] += 1;
								}
							else if (result0.error_code == CPARSE_INVALID_ETHER_TYPE ||
									 result0.error_code == CPARSE_UNSUPPORTED_PROTOCOL)
								{
									/* The packet is a valid non-UDP frame (e.g., ARP, ICMP, TCP).
									 * This is not an error for our classifier, we simply pass it
									 * to VPP's normal pipeline without incrementing error counters,
									 * preserving L2/L3 connectivity. */
									vnet_feature_next (&next0, b0);
								}
							else
								{
									next0 = RUST_CLASSIFY_NEXT_DROP;
									error_counts[result0.error_code] += 1;
								}
						}

					if (PREDICT_FALSE ((node->flags & VLIB_NODE_FLAG_TRACE) &&
									   (b0->flags & VLIB_BUFFER_IS_TRACED)))
						{
							rust_classify_trace_t *t = vlib_add_trace (vm, node, b0, sizeof (*t));
							t->sw_if_index = vnet_buffer (b0)->sw_if_index[VLIB_RX];
							t->next_index = next0;

							if (PREDICT_FALSE (rust_classify_passthrough))
								{
									t->is_valid = 1; /* Mark as valid for trace clarity */
									t->protocol = 0;
									t->dest_port = 0;
									t->error_code = CPARSE_OK;
								}
							else
								{
									t->is_valid = result0.is_valid;
									t->protocol = result0.protocol;
									t->dest_port = result0.dest_port;
									t->error_code = result0.error_code;
								}
						}

					vlib_validate_buffer_enqueue_x1 (vm, node, next_index, to_next, n_left_to_next,
													 bi0, next0);
				}

			vlib_put_next_frame (vm, node, next_index, n_left_to_next);
		}

	for (int i = 0; i < RUST_CLASSIFY_N_ERROR; i++)
		{
			if (error_counts[i] > 0)
				vlib_node_increment_counter (vm, node->node_index, i, error_counts[i]);
		}

	return frame->n_vectors;
}

/* *INDENT-OFF* */

/* Register the node as a candidate feature on device-input, so it sees
 * complete frames (Ethernet header intact) before ethernet-input */
VNET_FEATURE_INIT (rust_classify_feature, static) = {
	.arc_name = "device-input",
	.node_name = "rust-classify",
	.runs_before = VNET_FEATURES ("ethernet-input"),
};

VLIB_REGISTER_NODE(rust_classify_node) = {
    .name = "rust-classify",
    .vector_size = sizeof(u32),
    .format_trace = format_rust_classify_trace,
    .type = VLIB_NODE_TYPE_INTERNAL,

    .n_errors = ARRAY_LEN(rust_classify_error_strings),
    .error_strings = rust_classify_error_strings,

    .n_next_nodes = RUST_CLASSIFY_N_NEXT,
    .next_nodes =
        {
            [RUST_CLASSIFY_NEXT_DROP] = "error-drop",
        },
};

/* *INDENT-ON* */
