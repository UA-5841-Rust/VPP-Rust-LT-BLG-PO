#ifndef RUST_CLASSIFY_BENCH_H
#define RUST_CLASSIFY_BENCH_H
/* Immutable feature configuration; CLI changes run under the worker barrier. */
typedef struct {
  u32 tx_sw_if_index;
  u32 passthrough;
} rust_classify_config_t;
#endif
