#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
workers=${1:-1}
queues=${2:-1}
pin=${3:-pinned}
[[ $workers == 1 || $workers == 2 ]] || exit 2
[[ $queues == 1 || $queues == 2 ]] || exit 2
[[ $pin == pinned || $pin == unpinned ]] || exit 2
bash "$BENCH/teardown.sh"
mkdir -p "$RUN"
for side in a b; do
  ip netns add "rc-$side"
  ip netns exec "rc-$side" sysctl -qw net.ipv6.conf.all.disable_ipv6=1
  ip link add "rc-$side-vpp" type veth peer name "rc-$side-ns"
  sysctl -qw "net.ipv6.conf.rc-$side-vpp.disable_ipv6=1"
  ip link set "rc-$side-ns" netns "rc-$side"
  ip link set "rc-$side-vpp" up
  ip netns exec "rc-$side" ip link set lo up
  ip netns exec "rc-$side" ip link set "rc-$side-ns" up
done
ip netns exec rc-a ip addr add 10.44.0.1/24 dev rc-a-ns
ip netns exec rc-b ip addr add 10.44.0.2/24 dev rc-b-ns
cpu=''
if [[ $pin == pinned ]]; then
  corelist=2
  [[ $workers == 2 ]] && corelist=2,4
  cpu="main-core 0 corelist-workers $corelist"
else
  cpu="workers $workers"
fi
cat > "$RUN/startup.conf" <<EOF
unix { nodaemon cli-listen $RUN/cli.sock log $RUN/vpp.log }
api-segment { prefix rc-week4 }
statseg { socket-name $RUN/stats.sock }
cpu { $cpu }
plugins {
  path $BENCH/build/release:$VPP_DIR/build-root/install-vpp-native/vpp/lib/x86_64-linux-gnu/vpp_plugins
  plugin default { disable }
  plugin af_packet_plugin.so { enable }
  plugin rust_classify_plugin.so { enable }
}
EOF
"$VPP_BIN/vpp" -c "$RUN/startup.conf" > "$BENCH/console.log" 2>&1 &
echo $! > "$RUN/vpp.pid"
for _ in {1..100}; do
  [[ -S "$RUN/cli.sock" ]] && break
  kill -0 "$(cat "$RUN/vpp.pid")" || { cat "$RUN/console.log"; exit 1; }
  sleep .1
done
if [[ $pin == unpinned ]]; then
  for thread in /proc/"$(cat "$RUN/vpp.pid")"/task/*; do
    taskset -pc "$(awk '/Cpus_allowed_list/ {print $2}' /proc/self/status)" "${thread##*/}" >/dev/null
  done
fi
cli create host-interface name rc-a-vpp num-rx-queues "$queues" num-tx-queues 1
cli create host-interface name rc-b-vpp num-rx-queues "$queues" num-tx-queues 1
cli set interface state host-rc-a-vpp up
cli set interface state host-rc-b-vpp up
cli set interface l2 xconnect host-rc-a-vpp host-rc-b-vpp
cli set interface l2 xconnect host-rc-b-vpp host-rc-a-vpp
cli rust classify host-rc-a-vpp to host-rc-b-vpp
cli set interface rx-placement worker 0 queue 0 host-rc-a-vpp
cli set interface rx-placement worker 0 queue 0 host-rc-b-vpp
if [[ $queues == 2 ]]; then
  second=0
  [[ $workers == 2 ]] && second=1
  cli set interface rx-placement worker "$second" queue 1 host-rc-a-vpp
  cli set interface rx-placement worker "$second" queue 1 host-rc-b-vpp
fi
cli show interface rx-placement
