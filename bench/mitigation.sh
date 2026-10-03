#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
tag=${1:-$(date +%Y%m%dT%H%M%S)}
trap 'bash "$BENCH/teardown.sh"' EXIT
bash "$BENCH/setup.sh" 1 1 pinned
bash "$BENCH/sanity.sh" "$BENCH/results/$tag-sanity"
# Alternate before/after runs, rather than treating one noisy run as proof.
for repetition in 1 2 3; do
  bash "$BENCH/run.sh" iperf classify 1000000000 "$tag-default-$repetition"
  SOCKET_BUFFER=212992 bash "$BENCH/run.sh" iperf classify 1000000000 "$tag-buffer-$repetition"
done
