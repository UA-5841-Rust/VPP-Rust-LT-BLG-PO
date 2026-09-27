#!/bin/bash

echo "Cleaning up old topology..."
ip netns del ns-load 2>/dev/null || true
ip netns del ns-sink 2>/dev/null || true
ip link del vpp-load 2>/dev/null || true
ip link del vpp-sink 2>/dev/null || true

set -e

echo "Creating namespaces..."
ip netns add ns-load
ip netns add ns-sink

echo "Creating veth interfaces..."
ip link add name vpp-load type veth peer name eth-load netns ns-load
ip link add name vpp-sink type veth peer name eth-sink netns ns-sink

# Set static MAC addresses for the namespaces (executed inside the namespace)
ip -n ns-load link set dev eth-load address 02:00:00:00:01:02
ip -n ns-sink link set dev eth-sink address 02:00:00:00:02:02

echo "Bringing up interfaces for VPP..."
ip link set dev vpp-load up
ip link set dev vpp-sink up

echo "Configuring ns-load..."
ip -n ns-load addr add 10.10.1.2/24 dev eth-load
ip -n ns-load link set dev eth-load up
ip -n ns-load link set dev lo up
ip -n ns-load route add default via 10.10.1.1
# Static ARP to reach VPP
ip -n ns-load neigh add 10.10.1.1 lladdr 02:00:00:00:01:01 dev eth-load

echo "Configuring ns-sink..."
ip -n ns-sink addr add 10.10.2.2/24 dev eth-sink
ip -n ns-sink link set dev eth-sink up
ip -n ns-sink link set dev lo up
ip -n ns-sink route add default via 10.10.2.1
# Static ARP to reach VPP
ip -n ns-sink neigh add 10.10.2.1 lladdr 02:00:00:00:02:01 dev eth-sink

echo "Topology successfully created."