#!/bin/bash
# Description: Captures VPP metrics during load testing.

set -euo pipefail

# Allow overriding paths via environment variables. 
# Defaults to global 'vppctl' if not explicitly provided.
VPPCTL_BIN="${VPPCTL:-vppctl}"
CLI_SOCK="${VPP_SOCK:-/run/vpp/cli.sock}"

# Resolve directory dynamically based on script location
OUTPUT_DIR="$(dirname "$0")/results"
mkdir -p "$OUTPUT_DIR"

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
REPORT_FILE="$OUTPUT_DIR/snapshot_$TIMESTAMP.txt"

echo "=== VPP Snapshot at $TIMESTAMP ===" > "$REPORT_FILE"

echo -e "\n--- SHOW RUN ---" >> "$REPORT_FILE"
sudo "$VPPCTL_BIN" -s "$CLI_SOCK" show run >> "$REPORT_FILE"

echo -e "\n--- SHOW ERRORS ---" >> "$REPORT_FILE"
sudo "$VPPCTL_BIN" -s "$CLI_SOCK" show errors >> "$REPORT_FILE"

echo -e "\n--- SHOW HARDWARE-INTERFACES ---" >> "$REPORT_FILE"
sudo "$VPPCTL_BIN" -s "$CLI_SOCK" show hardware-interfaces >> "$REPORT_FILE"

echo "Snapshot saved to $REPORT_FILE"