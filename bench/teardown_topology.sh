#!/usr/bin/env bash
set -euo pipefail

# Removes the bench topology created by setup-topology.sh. Safe to run
# even if the topology doesn't exist (e.g. after a failed setup run).
#
# If VPP is running with host-interfaces bound to vpp-left/vpp-right,
# stop it before tearing down — deleting the namespaces removes those
# kernel interfaces out from under a live af_packet binding.

NS_LEFT="ns-left"
NS_RIGHT="ns-right"

require_root() {
	if [[ "$(id -u)" -ne 0 ]]; then
		echo "must run as root" >&2
		exit 1
	fi
}

# Deleting a namespace also removes the peer end of any veth pair that
# was left in the root namespace — no separate interface cleanup needed.
delete_if_present() {
	local netns=$1
	if ip netns list | grep -qw "$netns"; then
		ip netns del "$netns"
		echo "removed ${netns}"
	else
		echo "${netns} not present, skipping"
	fi
}

main() {
	require_root
	delete_if_present "$NS_LEFT"
	delete_if_present "$NS_RIGHT"
}

main "$@"
