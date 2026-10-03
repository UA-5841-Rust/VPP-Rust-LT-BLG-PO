/* Educational Ethernet/IPv4/UDP classifier. See docs/ffi-boundary.md. */
#include <vlib/vlib.h>
#include <vnet/vnet.h>
#include <vnet/ethernet/ethernet.h>
#include <vnet/feature/feature.h>
#include "network_parser.h"
#include "bench.h"

typedef enum
{
  RUST_CLASSIFY_ERROR_MALFORMED,
  RUST_CLASSIFY_ERROR_UNSUPPORTED,
  RUST_CLASSIFY_ERROR_FORWARDED,
  RUST_CLASSIFY_ERROR_CHAINED,
  RUST_CLASSIFY_ERROR_DROPPED,
  RUST_CLASSIFY_N_ERROR,
} rust_classify_error_t;

static char *rust_classify_error_strings[] = {
  "malformed_packet", "unsupported_protocol", "forwarded_ok",
  "chained_buffer", "dropped",
};

typedef enum
{
  RUST_CLASSIFY_NEXT_DROP,
  RUST_CLASSIFY_NEXT_FORWARD,
  RUST_CLASSIFY_N_NEXT,
} rust_classify_next_t;

typedef struct
{
  ClassifyResult result;
  u32 next_index;
  u32 rx_sw_if_index;
  u8 chained;
  u8 passthrough;
  u32 tx_sw_if_index;
} rust_classify_trace_t;

static u8 *
format_rust_classify_trace (u8 *s, va_list *args)
{
  CLIB_UNUSED (vlib_main_t *vm) = va_arg (*args, vlib_main_t *);
  CLIB_UNUSED (vlib_node_t *node) = va_arg (*args, vlib_node_t *);
  rust_classify_trace_t *t = va_arg (*args, rust_classify_trace_t *);
  return format (s, "rust-classify: protocol %u port %u valid %u error %u "
                   "next %u rx %u chained %u passthrough %u tx %u",
                 t->result.protocol, t->result.dest_port, t->result.is_valid,
                 t->result.error_code, t->next_index, t->rx_sw_if_index,
                 t->chained, t->passthrough, t->tx_sw_if_index);
}

VLIB_NODE_FN (rust_classify_node)
(vlib_main_t *vm, vlib_node_runtime_t *node, vlib_frame_t *frame)
{
  u32 *from = vlib_frame_vector_args (frame);
  vlib_buffer_t *buffers[VLIB_FRAME_SIZE];
  u16 nexts[VLIB_FRAME_SIZE];
  u32 counts[RUST_CLASSIFY_N_ERROR] = { 0 };
  vlib_get_buffers (vm, from, buffers, frame->n_vectors);

  for (u32 i = 0; i < frame->n_vectors; i++)
    {
      vlib_buffer_t *b = buffers[i];
      u32 feature_next;
      rust_classify_config_t *config = vnet_feature_next_with_data (
        &feature_next, b, sizeof (*config));
      /* Bench mode lets ARP and TCP control traffic follow the normal L2
       * cross-connect. UDP alone is consumed by the classifier. All offsets
       * below are guarded; Rust remains responsible for UDP validation. */
      const u8 *packet = vlib_buffer_get_current (b);
      if (config->tx_sw_if_index != ~0 && b->current_length >= 34 &&
          ((packet[12] == 8 && packet[13] == 6) ||
           (packet[12] == 8 && packet[13] == 0 &&
            (packet[23] == 6 || packet[23] == 1))))
        {
          nexts[i] = feature_next;
          continue;
        }
      vnet_buffer (b)->sw_if_index[VLIB_TX] = config->tx_sw_if_index;
      ClassifyResult result = { 0, 0, 0, PACKET_MALFORMED };
      u32 category;
      u8 chained = (b->flags & VLIB_BUFFER_NEXT_PRESENT) != 0;
      nexts[i] = RUST_CLASSIFY_NEXT_DROP;

      if (PREDICT_FALSE (chained))
        {
          /* A chain is not one contiguous Rust slice. Never substitute total
           * chain length for current_length, and never linearize/copy here. */
          category = RUST_CLASSIFY_ERROR_UNSUPPORTED;
          counts[RUST_CLASSIFY_ERROR_CHAINED]++;
        }
      else if (config->passthrough)
        {
          result = (ClassifyResult) {
            .is_valid = 1, .protocol = 1, .dest_port = 0,
            .error_code = PACKET_OK,
          };
          category = RUST_CLASSIFY_ERROR_FORWARDED;
          nexts[i] = RUST_CLASSIFY_NEXT_FORWARD;
        }
      else
        {
          /* FFI SAFETY: device-input supplies a live VPP-owned buffer at its
           * Ethernet header. current_length covers only this readable segment;
           * chains are rejected above. The worker owns the buffer during this
           * synchronous call and neither frees nor mutates it. Rust borrows
           * bytes only until return and bounds-checks all packet fields. */
          result = packet_classify (vlib_buffer_get_current (b),
                                    b->current_length);
          if (result.is_valid && result.protocol == 1 &&
              result.error_code == PACKET_OK)
            {
              category = RUST_CLASSIFY_ERROR_FORWARDED;
              nexts[i] = RUST_CLASSIFY_NEXT_FORWARD;
            }
          else if (result.error_code == PACKET_INVALID_ETHERTYPE ||
                   result.error_code == PACKET_UNSUPPORTED_PROTOCOL ||
                   result.error_code == PACKET_UNSUPPORTED_FRAGMENT)
            category = RUST_CLASSIFY_ERROR_UNSUPPORTED;
          else
            category = RUST_CLASSIFY_ERROR_MALFORMED;
        }

      counts[category]++;
      /* error-drop accounts only the separate aggregate dropped counter.
       * Assigning a classification error here would count it twice, because
       * classification counters are incremented in batches below. */
      b->error = nexts[i] == RUST_CLASSIFY_NEXT_DROP ?
                   node->errors[RUST_CLASSIFY_ERROR_DROPPED] : 0;
      if (PREDICT_FALSE ((node->flags & VLIB_NODE_FLAG_TRACE) &&
                        (b->flags & VLIB_BUFFER_IS_TRACED)))
        {
          rust_classify_trace_t *t = vlib_add_trace (vm, node, b, sizeof (*t));
          t->result = result;
          t->next_index = nexts[i];
          t->rx_sw_if_index = vnet_buffer (b)->sw_if_index[VLIB_RX];
          t->chained = chained;
          t->passthrough = config->passthrough;
          t->tx_sw_if_index = config->tx_sw_if_index;
        }
    }

  for (u32 i = 0; i < RUST_CLASSIFY_ERROR_DROPPED; i++)
    if (counts[i])
      vlib_node_increment_counter (vm, node->node_index, i, counts[i]);
  vlib_buffer_enqueue_to_next (vm, node, from, nexts, frame->n_vectors);
  return frame->n_vectors;
}

VLIB_REGISTER_NODE (rust_classify_node) = {
  .name = "rust-classify-node",
  .vector_size = sizeof (u32),
  .type = VLIB_NODE_TYPE_INTERNAL,
  .format_trace = format_rust_classify_trace,
  .n_errors = RUST_CLASSIFY_N_ERROR,
  .error_strings = rust_classify_error_strings,
  .n_next_nodes = RUST_CLASSIFY_N_NEXT,
  .next_nodes = {
    [RUST_CLASSIFY_NEXT_DROP] = "error-drop",
    [RUST_CLASSIFY_NEXT_FORWARD] = "rust-classify-forward",
  },
};

/* Minimal observable next node: echo an accepted frame to its ingress port.
 * IP/UDP fields and payload stay unchanged; only Ethernet MACs are exchanged. */
VLIB_NODE_FN (rust_classify_forward_node)
(vlib_main_t *vm, vlib_node_runtime_t *node, vlib_frame_t *frame)
{
  u32 *from = vlib_frame_vector_args (frame);
  u16 nexts[VLIB_FRAME_SIZE] = { 0 };
  for (u32 i = 0; i < frame->n_vectors; i++)
    {
      vlib_buffer_t *b = vlib_get_buffer (vm, from[i]);
      /* SAFETY: only the classifier feeds this node in the supported graph.
       * Retain a length check before MAC accesses even for accidental direct
       * packet-generator injection. VPP owns the writable segment. */
      if (PREDICT_FALSE (b->current_length < sizeof (ethernet_header_t)))
        {
          nexts[i] = 1;
          continue;
        }
      if (vnet_buffer (b)->sw_if_index[VLIB_TX] == ~0)
        {
          u8 *data = vlib_buffer_get_current (b);
          for (u32 j = 0; j < 6; j++)
            {
              u8 tmp = data[j];
              data[j] = data[j + 6];
              data[j + 6] = tmp;
            }
          vnet_buffer (b)->sw_if_index[VLIB_TX] =
            vnet_buffer (b)->sw_if_index[VLIB_RX];
        }
    }
  vlib_buffer_enqueue_to_next (vm, node, from, nexts, frame->n_vectors);
  return frame->n_vectors;
}

VLIB_REGISTER_NODE (rust_classify_forward_node) = {
  .name = "rust-classify-forward",
  .vector_size = sizeof (u32),
  .type = VLIB_NODE_TYPE_INTERNAL,
  .n_next_nodes = 2,
  .next_nodes = { [0] = "interface-output", [1] = "error-drop" },
};
