.PHONY: test generate_header

test:
	cargo test

generate_header:
	cbindgen --config cbindgen.toml --crate network_parser --output include/network_parser.h
