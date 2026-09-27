#!/bin/bash

echo "Cleaning up old VPP interfaces..."
vppctl delete host-interface name vpp-load 2>/dev/null || true
vppctl delete host-interface name vpp-sink 2>/dev/null || true

echo "Creating host interfaces in VPP..."
vppctl create host-interface name vpp-load
vppctl create host-interface name vpp-sink

# Set static MAC addresses for VPP interfaces BEFORE bringing them up
vppctl set interface mac address host-vpp-load 02:00:00:00:01:01
vppctl set interface mac address host-vpp-sink 02:00:00:00:02:01

echo "Setting interfaces up in VPP..."
vppctl set interface state host-vpp-load up
vppctl set interface state host-vpp-sink up

echo "Assigning IP addresses..."
vppctl set interface ip address host-vpp-load 10.10.1.1/24
vppctl set interface ip address host-vpp-sink 10.10.2.1/24

# Static ARP entries inside VPP
vppctl set ip neighbor host-vpp-load 10.10.1.2 02:00:00:00:01:02
vppctl set ip neighbor host-vpp-sink 10.10.2.2 02:00:00:00:02:02

echo "Attaching rust-classify-node..."
vppctl set interface feature host-vpp-load rust-classify-node arc device-input

echo "Configuring trace..."
vppctl trace add af-packet-input 100
vppctl clear errors
vppctl clear run

echo "Configuring rx-placement..."
vppctl set interface rx-placement host-vpp-load queue 0 worker 0

echo "VPP successfully configured."