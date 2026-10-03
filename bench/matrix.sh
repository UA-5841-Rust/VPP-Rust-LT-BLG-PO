#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
tag=${1:-$(date +%Y%m%dT%H%M%S)}
trap 'bash "$BENCH/teardown.sh"' EXIT
bash "$BENCH/setup.sh" 1 1 pinned
bash "$BENCH/sanity.sh" "$BENCH/results/$tag-sanity"
cli clear errors
cli clear runtime
cli clear interfaces
sleep 2
bash "$BENCH/snapshot.sh" "$BENCH/results/$tag-idle"
for mode in classify passthrough; do
  for rate in 1000000 50000000 1000000000; do
    PROFILE=0
    [[ $mode == classify && $rate == 1000000000 ]] && PROFILE=1
    PROFILE=$PROFILE bash "$BENCH/run.sh" iperf "$mode" "$rate" "$tag-iperf-$mode-$rate"
  done
  for rate in 1000 20000 200000; do
    bash "$BENCH/run.sh" custom "$mode" "$rate" "$tag-custom-$mode-$rate"
  done
done
# Separate queue-count effects from worker-count effects.
bash "$BENCH/setup.sh" 1 2 pinned
bash "$BENCH/run.sh" custom classify 200000 "$tag-one-worker-two-queues"
bash "$BENCH/setup.sh" 2 2 pinned
bash "$BENCH/run.sh" custom classify 200000 "$tag-two-workers-two-queues"
bash "$BENCH/setup.sh" 1 1 unpinned
bash "$BENCH/run.sh" iperf classify 1000000000 "$tag-unpinned"
