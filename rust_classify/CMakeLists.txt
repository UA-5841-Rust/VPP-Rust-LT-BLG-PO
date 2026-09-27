add_vpp_plugin(rust_classify
  SOURCES
  rust_classify.c
  rust_classify_node.c
)

set(NETWORK_PARSER_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../network_parser" CACHE PATH "Path to the network_parser crate")

set(NETWORK_PARSER_LIB ${NETWORK_PARSER_DIR}/target/release/libnetwork_parser.a)

add_custom_command(
  OUTPUT ${NETWORK_PARSER_LIB}
  COMMAND cargo build --release --manifest-path ${NETWORK_PARSER_DIR}/Cargo.toml
)

add_custom_target(network_parser_rust ALL
  DEPENDS ${NETWORK_PARSER_LIB})

add_dependencies(rust_classify_plugin network_parser_rust)

target_link_libraries(rust_classify_plugin
  ${NETWORK_PARSER_LIB}
  pthread
  dl
  m
)
