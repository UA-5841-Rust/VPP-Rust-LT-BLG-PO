#!/usr/bin/env bash
set -euo pipefail

# Bench topology (variant 1): two netns connected through VPP running in
# the root netns. Each veth pair has one end in a namespace and one end
# left in the root netns for VPP's af_packet host-interfaces to bind to.
#
#   ns-left --veth-left----vpp-left--[ VPP ]--vpp-right----veth-right-- ns-right
#           10.10.1.2/24  10.10.1.1/24      10.10.2.1/24  10.10.2.2/24
#
# vpp-left/vpp-right get no kernel IP — VPP is the only L3 participant
# on those addresses.

NS_LEFT="ns-left"
NS_RIGHT="ns-right"
VETH_LEFT="veth-left"
VPP_LEFT="vpp-left"
VETH_RIGHT="veth-right"
VPP_RIGHT="vpp-right"
LEFT_ADDR="10.10.1.2/24"
LEFT_GW="10.10.1.1"
RIGHT_ADDR="10.10.2.2/24"
RIGHT_GW="10.10.2.1"

require_root() {
	if [[ "$(id -u)" -ne 0 ]]; then
		echo "must run as root (netns/veth/ethtool require it)" >&2
		exit 1
	fi
}

# Refuses to run on top of an existing topology instead of silently
# tearing it down — run teardown-topology.sh explicitly first.
check_clean_state() {
	if ip netns list | grep -qw "$NS_LEFT" || ip netns list | grep -qw "$NS_RIGHT"; then
		echo "topology already exists — run teardown-topology.sh first" >&2
		exit 1
	fi
}

# Creates one namespace, its veth pair, addressing, and offload tuning.
# args: netns_name ns_side_if root_side_if ns_addr_cidr gateway_ip
create_side() {
	local netns=$1 ns_if=$2 root_if=$3 addr=$4 gw=$5

	ip netns add "$netns"
	ip link add "$ns_if" type veth peer name "$root_if"
	ip link set "$ns_if" netns "$netns"

	# Enable promiscuous mode to bypass MAC-spoofing drops on af_packet TX.
	ip link set "$root_if" promisc on
	ip netns exec "$netns" ip link set "$ns_if" promisc on

	ip link set "$root_if" up
	ip netns exec "$netns" ip link set lo up
	ip netns exec "$netns" ip link set "$ns_if" up
	ip netns exec "$netns" ip addr add "$addr" dev "$ns_if"
	ip netns exec "$netns" ip route add default via "$gw" dev "$ns_if"

	# NOTE: kernel checksum/segmentation offloads are intentionally left ON.
	# Disabling them (ethtool -K tx/rx/gso/gro/tso off) broke forwarding on
	# this platform — and it is unnecessary here: VPP's af_packet interface
	# has cksum-gso-enabled in its features, so it negotiates offloads with
	# the kernel itself (see show hardware-interfaces output).
}

main() {
	require_root
	check_clean_state
	create_side "$NS_LEFT" "$VETH_LEFT" "$VPP_LEFT" "$LEFT_ADDR" "$LEFT_GW"
	create_side "$NS_RIGHT" "$VETH_RIGHT" "$VPP_RIGHT" "$RIGHT_ADDR" "$RIGHT_GW"
	echo "topology ready: ${NS_LEFT} (${LEFT_ADDR}) <-> VPP <-> ${NS_RIGHT} (${RIGHT_ADDR})"
	echo "root-ns interfaces for VPP host-interfaces: ${VPP_LEFT}, ${VPP_RIGHT}"
}

main "$@"
