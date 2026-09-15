#include <vnet/vnet.h>
#include <vnet/plugin/plugin.h>
#include <rust_classify/rust_classify.h>

#include <network_parser.h>
#include <vpp/app/version.h>

rust_classify_main_t rust_classify_main;

/* Initialize passthrough to 0 (disabled by default).
 * This allows Part B to measure baseline vs pure-forwarding overhead. */
volatile u8 rust_classify_passthrough = 0;

static clib_error_t *
rust_classify_init (vlib_main_t *vm)
{
	rust_classify_main_t *rmp = &rust_classify_main;
	rmp->vnet_main = vnet_get_main ();

	// linker function
	struct ClassifyResult link_check = packet_classify (0, 0);
	(void) link_check;

	return 0;
}

VLIB_INIT_FUNCTION (rust_classify_init);

/* CLI command to toggle passthrough mode at runtime.
 * Usage: rust-classify passthrough on|off
 * This avoids rebuilding VPP just to measure FFI overhead. */
static clib_error_t *
rust_classify_passthrough_command_fn (vlib_main_t *vm, unformat_input_t *input,
									  vlib_cli_command_t *cmd)
{
	while (unformat_check_input (input) != UNFORMAT_END_OF_INPUT)
		{
			if (unformat (input, "on"))
				rust_classify_passthrough = 1;
			else if (unformat (input, "off"))
				rust_classify_passthrough = 0;
			else
				return clib_error_return (0, "unknown input `%U'", format_unformat_error, input);
		}
	return 0;
}

VLIB_CLI_COMMAND (rust_classify_passthrough_cmd, static) = {
	.path = "rust-classify passthrough",
	.short_help = "rust-classify passthrough on|off",
	.function = rust_classify_passthrough_command_fn,
};

/* *INDENT-OFF* */
VLIB_PLUGIN_REGISTER () = {
	.version = VPP_BUILD_VER,
	.description = "Rust-based UDP packet classification node",
};
/* *INDENT-ON* */
