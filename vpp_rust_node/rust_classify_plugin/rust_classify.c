#include <vlib/vlib.h>
#include <vnet/vnet.h>
#include <vnet/plugin/plugin.h>
#include "network_parser.h"

#define foreach_rust_classify_error \
_(FORWARDED_OK, "Valid UDP packets forwarded") \
_(MALFORMED_PACKET, "Malformed packets dropped") \
_(UNSUPPORTED_PROTOCOL, "Unsupported protocol dropped")

typedef enum {
    #define _(sym,str) RUST_CLASSIFY_ERROR_##sym,
      foreach_rust_classify_error
    #undef _
      RUST_CLASSIFY_N_ERROR,
    } rust_classify_error_t;
    
    static char * rust_classify_error_strings[] = {
    #define _(sym,string) string,
      foreach_rust_classify_error
    #undef _
    };

/* Define next nodes */
typedef enum {
    RUST_CLASSIFY_NEXT_DROP,
    RUST_CLASSIFY_NEXT_FORWARD,
    RUST_CLASSIFY_N_NEXT,
} rust_classify_next_t;

typedef struct {
    u8 protocol;
    u16 dest_port;
    u32 error_code;
    bool is_valid;
} rust_classify_trace_t;

static u8 *
format_rust_classify_trace (u8 * s, va_list * args)
{
    CLIB_UNUSED (vlib_main_t * vm) = va_arg (*args, vlib_main_t *);
    CLIB_UNUSED (vlib_node_t * node) = va_arg (*args, vlib_node_t *);
    rust_classify_trace_t * t = va_arg (*args, rust_classify_trace_t *);

    s = format (s, "RUST-CLASSIFY: valid %d, proto %d, port %d, error code %d",
                t->is_valid, t->protocol, t->dest_port, t->error_code);
    return s;
}

/* Main dispatch function for the node */
static uword
rust_classify_node_fn (vlib_main_t * vm,
                       vlib_node_runtime_t * node,
                       vlib_frame_t * frame)
{
    u32 n_left_from, * from, * to_next;
    rust_classify_next_t next_index;

    u32 forwarded_ok = 0;
    u32 malformed_packet = 0;
    u32 unsupported_protocol = 0;

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
            vlib_buffer_t * b0;
            u32 next0 = RUST_CLASSIFY_NEXT_DROP;

            bi0 = to_next[0] = from[0];
            from += 1;
            to_next += 1;
            n_left_from -= 1;
            n_left_to_next -= 1;

            b0 = vlib_get_buffer (vm, bi0);

            u8 * data = vlib_buffer_get_current (b0);
            u32 len = b0->current_length;

            /*
             * UNSAFE BOUNDARY JUSTIFICATION
             * - Why unsafe is required: Passing raw memory pointers (u8*) and length across the FFI boundary to Rust.
             * - Assumptions: 'data' points to a valid, contiguous memory block owned by VPP. 'len' correctly reflects the initialized packet payload size.
             * - Validation: vlib_buffer_get_current() and b0->current_length are guaranteed by VPP's memory manager. The Rust parser safely validates length boundaries before accessing protocol headers. No payload copies occur.
             */

            ClassifyResult res = packet_classify(data, len);

            if (res.is_valid) {
                next0 = RUST_CLASSIFY_NEXT_FORWARD;
                forwarded_ok++;
            } else {
                next0 = RUST_CLASSIFY_NEXT_DROP;
                const u32 unsupported_protocol_error_code = 7; /* ParseError::UnsupportedProtocol */
                if (res.error_code == unsupported_protocol_error_code) {
                    unsupported_protocol++;
                } else {
                    malformed_packet++;
                }
            }

            /*For Tracing*/
            if (PREDICT_FALSE ((node->flags & VLIB_NODE_FLAG_TRACE) && (b0->flags & VLIB_BUFFER_IS_TRACED))) {
                rust_classify_trace_t *t =
                    vlib_add_trace (vm, node, b0, sizeof (*t));
                t->is_valid = res.is_valid;
                t->protocol = res.protocol;
                t->dest_port = res.dest_port;
                t->error_code = res.error_code;
            }

            vlib_validate_buffer_enqueue_x1 (vm, node, next_index,
                                             to_next, n_left_to_next,
                                             bi0, next0);
        }

        vlib_put_next_frame (vm, node, next_index, n_left_to_next);
    }

    /*Batch incrementing counters after vector processing*/
    if (forwarded_ok)
        vlib_node_increment_counter (vm, node->node_index, RUST_CLASSIFY_ERROR_FORWARDED_OK, forwarded_ok);
    if (malformed_packet)
        vlib_node_increment_counter (vm, node->node_index, RUST_CLASSIFY_ERROR_MALFORMED_PACKET, malformed_packet);
    if (unsupported_protocol)
        vlib_node_increment_counter (vm, node->node_index, RUST_CLASSIFY_ERROR_UNSUPPORTED_PROTOCOL, unsupported_protocol);
    
    return frame->n_vectors;
}

/*
 * Registers the node as a feature on the device-input arc.
 * Intercepts raw incoming frames (with Ethernet headers) 
 * before they reach standard L2 processing.
 */
VNET_FEATURE_INIT (rust_classify_feature, static) = {
    .arc_name = "device-input",
    .node_name = "rust-classify-node",
    .runs_before = VNET_FEATURES ("ethernet-input"),
};


/* Register the node */
VLIB_REGISTER_NODE (rust_classify_node) = {
    .function = rust_classify_node_fn,
    .name = "rust-classify-node",
    .vector_size = sizeof (u32),
    .format_trace = format_rust_classify_trace,
    .type = VLIB_NODE_TYPE_INTERNAL,
    .n_errors = ARRAY_LEN(rust_classify_error_strings),
    .error_strings = rust_classify_error_strings,
    .n_next_nodes = RUST_CLASSIFY_N_NEXT,
    .next_nodes = {
        [RUST_CLASSIFY_NEXT_DROP] = "error-drop",
        [RUST_CLASSIFY_NEXT_FORWARD] = "ethernet-input",
    },
};

/* Register the plugin */
VLIB_PLUGIN_REGISTER () = {
    .version = "1.0",
    .description = "Rust Packet Classification Plugin",
};