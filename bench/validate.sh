#!/usr/bin/env bash
source "$(dirname "$0")/common.sh"
need_root
trap 'bash "$BENCH/teardown.sh"' EXIT
bash "$BENCH/setup.sh" 1 1 pinned
python3 "$BENCH/validate.py" check --vppctl "$VPP_BIN/vppctl" --runtime "$RUN" \
  --output "$BENCH/results/integration"
bash "$BENCH/snapshot.sh" "$BENCH/results/integration/snapshot"
