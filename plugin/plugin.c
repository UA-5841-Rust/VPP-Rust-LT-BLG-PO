#include <vlib/vlib.h>
#include <vnet/vnet.h>
#include <vnet/feature/feature.h>
#include <vnet/plugin/plugin.h>
#include <vpp/app/version.h>
#include "bench.h"

/* CLI-owned copies are used to remove the exact opaque feature binding.
 * Workers read only VPP's immutable feature data, never these vectors. */
static rust_classify_config_t *interface_configs;
static u8 *interface_enabled;

VLIB_PLUGIN_REGISTER () = {
  .version = VPP_BUILD_VER,
  .description = "Zero-copy Rust Ethernet/IPv4/UDP classifier",
};

VNET_FEATURE_INIT (rust_classify_feature, static) = {
  .arc_name = "device-input",
  .node_name = "rust-classify-node",
  .runs_before = VNET_FEATURES ("ethernet-input"),
};

static clib_error_t *
rust_classify_command_fn (vlib_main_t *vm, unformat_input_t *input,
                         vlib_cli_command_t *cmd)
{
  vnet_main_t *vnm = vnet_get_main ();
  u32 sw_if_index = ~0;
  int enable = 1;
  int rv;
  rust_classify_config_t config = { ~0, 0 };
  while (unformat_check_input (input) != UNFORMAT_END_OF_INPUT)
    {
      if (unformat (input, "disable"))
        enable = 0;
      else if (unformat (input, "to %U", unformat_vnet_sw_interface, vnm,
                         &config.tx_sw_if_index))
        ;
      else if (unformat (input, "passthrough"))
        config.passthrough = 1;
      else if (unformat (input, "%U", unformat_vnet_sw_interface, vnm,
                         &sw_if_index))
        ;
      else
        return clib_error_return (0, "unknown input: %U", format_unformat_error,
                                  input);
    }
  if (sw_if_index == ~0)
    return clib_error_return (0, "specify an Ethernet hardware interface");
  vnet_sw_interface_t *sw = vnet_get_sw_interface (vnm, sw_if_index);
  if (sw->type != VNET_SW_INTERFACE_TYPE_HARDWARE)
    return clib_error_return (0, "subinterfaces are not supported");
  /* Re-enabling an existing feature does not replace its config data in VPP.
   * Remove the old binding under the CLI worker barrier before adding the
   * new immutable mode/egress configuration. */
  vec_validate (interface_configs, sw_if_index);
  vec_validate (interface_enabled, sw_if_index);
  if (interface_enabled[sw_if_index])
    {
      rv = vnet_feature_enable_disable ("device-input", "rust-classify-node",
                                        sw_if_index, 0,
                                        &interface_configs[sw_if_index],
                                        sizeof (config));
      if (rv)
        return clib_error_return (0, "feature reset failed: %d", rv);
      interface_enabled[sw_if_index] = 0;
    }
  if (!enable)
    return 0;
  rv = vnet_feature_enable_disable ("device-input", "rust-classify-node",
                                    sw_if_index, enable, &config, sizeof (config));
  if (rv)
    return clib_error_return (0, "feature enable/disable failed: %d", rv);
  interface_configs[sw_if_index] = config;
  interface_enabled[sw_if_index] = 1;
  return 0;
}

/* CLI execution uses the default worker barrier (not mp_safe). No mutable
 * plugin-global data is accessed from packet-processing workers. */
VLIB_CLI_COMMAND (rust_classify_command, static) = {
  .path = "rust classify",
  .short_help = "rust classify <interface> [to <egress>] [passthrough] [disable]",
  .function = rust_classify_command_fn,
};
