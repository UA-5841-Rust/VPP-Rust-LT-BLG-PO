# Prerequisite: build the Rust crate first, from network_parser/:
#   cargo build --release
# which produces target/release/libnetwork_parser.a.

# NETWORK_PARSER_DIR must point at that checkout. No default is guessed
# here (e.g. "assume it's a sibling directory of VPP") — this plugin
# lives inside the VPP tree, network_parser doesn't, so any relative path
# between them depends entirely on how *you* happened to lay out your
# checkouts. A guessed default doesn't fail when your layout differs from
# what was assumed — it just silently resolves to the wrong directory, or
# to nothing. Better to require it explicitly and fail loudly if it's
# missing than to guess wrong quietly:
#   cmake ... -DNETWORK_PARSER_DIR=/path/to/network_parser
set(NETWORK_PARSER_DIR "" CACHE PATH
    "Path to the network_parser Rust crate checkout (must contain target/release/libnetwork_parser.a and include/network_parser.h)")

if (NOT NETWORK_PARSER_DIR)
  message(FATAL_ERROR
    "NETWORK_PARSER_DIR is not set. Pass "
    "-DNETWORK_PARSER_DIR=/path/to/network_parser when invoking cmake.")
endif ()

set(NETWORK_PARSER_LIB "${NETWORK_PARSER_DIR}/target/release/libnetwork_parser.a")
set(NETWORK_PARSER_INCLUDE "${NETWORK_PARSER_DIR}/include")

# Plain EXISTS checks, not find_library()/find_path(): those cache their
# result the first time they run, so if you configure before running
# `cargo build --release`, get a stale "not found", then build the Rust
# side and re-run cmake, find_library would still report the old cached
# miss. A plain EXISTS check re-evaluates on every configure, so it
# reflects reality right away.
if (NOT EXISTS "${NETWORK_PARSER_LIB}")
  message(FATAL_ERROR
    "libnetwork_parser.a not found at ${NETWORK_PARSER_LIB} — "
    "run `cargo build --release` in ${NETWORK_PARSER_DIR} first.")
endif ()

if (NOT EXISTS "${NETWORK_PARSER_INCLUDE}/network_parser.h")
  message(FATAL_ERROR
    "network_parser.h not found under ${NETWORK_PARSER_INCLUDE}")
endif ()

# IMPORTED target rather than splicing the raw .a path straight into
# add_vpp_plugin's LINK_LIBRARIES: this way INTERFACE_INCLUDE_DIRECTORIES
# travels automatically to whatever links against it, so we don't also
# have to separately call target_include_directories() on the internal
# target name add_vpp_plugin() generates (that name isn't part of its
# public contract and has drifted across VPP releases before).
add_library(network_parser STATIC IMPORTED)
set_target_properties(network_parser PROPERTIES
  IMPORTED_LOCATION "${NETWORK_PARSER_LIB}"
  INTERFACE_INCLUDE_DIRECTORIES "${NETWORK_PARSER_INCLUDE}"
)

add_vpp_plugin(rust_classify
  SOURCES
  rust_classify.c
  node.c

  LINK_LIBRARIES
  network_parser
  # Rust's std pulls these in on Linux; without them the final .so link
  # fails with undefined references even though packet_classify itself
  # resolves fine.
  pthread
  dl
  m
)
