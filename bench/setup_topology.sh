#!/bin/bash
# Description: Sets up the veth-based network topology (Variant 1) for VPP testing.
# Creates two namespaces (ns-gen, ns-sink) and links them to the host via veth pairs.

set -e 

echo "Cleaning up previous topology..."
ip netns del ns-gen 2>/dev/null || true
ip netns del ns-sink 2>/dev/null || true

echo "Creating namespaces..."
ip netns add ns-gen
ip netns add ns-sink

echo "Creating veth pairs..."
# veth-gen <---> veth-vpp1 (Generator to VPP)
ip link add veth-gen type veth peer name veth-vpp1
# veth-sink <---> veth-vpp2 (Sink to VPP)
ip link add veth-sink type veth peer name veth-vpp2

echo "Configuring Generator namespace (ns-gen)..."
ip link set veth-gen netns ns-gen
ip netns exec ns-gen ip addr add 10.10.1.2/24 dev veth-gen
ip netns exec ns-gen ip link set veth-gen up
# Route traffic for the sink subnet through VPP's interface
ip netns exec ns-gen ip route add 10.10.2.0/24 via 10.10.1.1

echo "Configuring Sink namespace (ns-sink)..."
ip link set veth-sink netns ns-sink
ip netns exec ns-sink ip addr add 10.10.2.2/24 dev veth-sink
ip netns exec ns-sink ip link set veth-sink up
# Route traffic for the sink subnet through VPP's interface
ip netns exec ns-sink ip route add 10.10.1.0/24 via 10.10.2.1

echo "Bringing up host VPP-facing interfaces..."
ip link set veth-vpp1 up
ip link set veth-vpp2 up

echo "Topology setup complete."