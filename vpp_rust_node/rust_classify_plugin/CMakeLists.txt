set(RUST_PROJ_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../network_parser)

if(CMAKE_BUILD_TYPE MATCHES "Release")
    set(CARGO_PROFILE "release")
    set(CARGO_ARGS "--release")
else()
    set(CARGO_PROFILE "debug")
    set(CARGO_ARGS "")
endif()

# Define the path to the compiled Rust static library artifact.
set(RUST_LIB ${RUST_PROJ_DIR}/target/${CARGO_PROFILE}/libnetwork_parser.a)

# Register a custom target to invoke the Cargo build system prior to C compilation.
add_custom_target(rust_network_parser_build ALL
    COMMAND cargo build ${CARGO_ARGS}
    WORKING_DIRECTORY ${RUST_PROJ_DIR}
    COMMENT "Invoking Cargo to build the network_parser library (${CARGO_PROFILE})"
)

add_vpp_plugin(rust_classify
    SOURCES rust_classify.c
    LINK_LIBRARIES ${RUST_LIB}
)

# Enforce build order: The Rust library must be built before the VPP plugin compiles.
add_dependencies(rust_classify rust_network_parser_build)