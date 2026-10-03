# Installed as a symlink at VPP/src/plugins/rust_classify. Resolve the real
# source directory so Cargo/includes remain in the student's repository.
get_filename_component(RUST_CLASSIFY_PLUGIN_DIR "${CMAKE_CURRENT_SOURCE_DIR}" REALPATH)
get_filename_component(RUST_CLASSIFY_ROOT "${RUST_CLASSIFY_PLUGIN_DIR}/.." ABSOLUTE)
find_program(RUST_CLASSIFY_CARGO cargo REQUIRED)
set(RUST_CLASSIFY_TARGET "${CMAKE_CURRENT_BINARY_DIR}/cargo-target")
set(RUST_CLASSIFY_ARCHIVE "${RUST_CLASSIFY_TARGET}/release/libnetwork_parser.a")
file(GLOB RUST_CLASSIFY_SOURCES CONFIGURE_DEPENDS "${RUST_CLASSIFY_ROOT}/src/*.rs")

add_custom_command(
  OUTPUT "${RUST_CLASSIFY_ARCHIVE}"
  COMMAND ${CMAKE_COMMAND} -E env "RUSTFLAGS=-C relocation-model=pic"
          ${RUST_CLASSIFY_CARGO} build --release --locked --offline
          --manifest-path "${RUST_CLASSIFY_ROOT}/Cargo.toml"
          --target-dir "${RUST_CLASSIFY_TARGET}"
  DEPENDS ${RUST_CLASSIFY_SOURCES}
          "${RUST_CLASSIFY_ROOT}/Cargo.toml" "${RUST_CLASSIFY_ROOT}/Cargo.lock"
  COMMENT "Building allocation-free Rust classifier"
  VERBATIM
)
add_custom_target(rust_classify_rust DEPENDS "${RUST_CLASSIFY_ARCHIVE}")
add_vpp_plugin(rust_classify
  SOURCES plugin.c node.c
  LINK_LIBRARIES "${RUST_CLASSIFY_ARCHIVE}" pthread dl m rt util
)
add_dependencies(rust_classify_plugin rust_classify_rust)
target_include_directories(rust_classify_plugin PRIVATE "${RUST_CLASSIFY_ROOT}/include")
