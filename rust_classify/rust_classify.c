#include <vlib/vlib.h>
#include <vnet/plugin/plugin.h>

VLIB_PLUGIN_REGISTER () = {
    .version = "1.0.0",
    .description = "Rust Packet Classification Plugin",
};