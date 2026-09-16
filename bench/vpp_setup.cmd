create host-interface name veth-vpp1 num-rx-queues 2
set interface ip address host-veth-vpp1 10.10.1.1/24
set interface state host-veth-vpp1 up

create host-interface name veth-vpp2
set interface ip address host-veth-vpp2 10.10.2.1/24
set interface state host-veth-vpp2 up

set interface feature host-veth-vpp1 rust-classify-node arc device-input