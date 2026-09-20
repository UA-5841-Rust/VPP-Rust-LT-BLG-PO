#!/usr/bin/env bash
set -euo pipefail

# Wraps any load-generation command with a consistent VPP metrics snapshot.
# It clears counters, runs the command, and captures VPP's state afterward.
# Contract: preserves the generator's exit code, but saves the snapshot
# regardless of failure (crucial for post-mortem analysis).
#
# Usage:
#   sudo env VPPCTL_BIN=<vppctl-path> ./snapshot_metrics.sh <label> -- <cmd...>
#
# Example:
# sudo env VPPCTL_BIN="$HOME/vpp/build-root/install-vpp-native/vpp/bin/vppctl" \
#     ./snapshot_metrics.sh iperf_test -- ip netns exec ns-left iperf3 -u -c 10.10.2.2 -b 50M
#
# Environment:
#   VPPCTL_BIN   (required) vppctl from the SAME build as the running VPP.
#                Mismatches cause binary-API segfaults, hence no default.
#   VPPCTL_SOCK  (optional) VPP CLI socket. Default: /run/vpp/cli.sock.
#                Must match "cli-listen" in the vpp startup .conf.
#   RESULTS_ROOT (optional) Output dir. Default: <script_dir>/../results/snapshot
#
# Safe to re-run for the same label: existing directories are overwritten.

# Resolve symlinks: works from any CWD and via symlinks (e.g. ~/bin).
SCRIPT_DIR="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"

VPPCTL_BIN="${VPPCTL_BIN:-}"
VPPCTL_SOCK="${VPPCTL_SOCK:-/run/vpp/cli.sock}"
RESULTS_ROOT="${RESULTS_ROOT:-${SCRIPT_DIR}/../results/snapshot}"

vppctl() {
	"$VPPCTL_BIN" -s "$VPPCTL_SOCK" "$@"
}

usage() {
	echo "usage: $0 <label> -- <load-generator command...>" >&2
	echo "       VPPCTL_BIN is required — see the header of this script" >&2
	exit 1
}

require_root() {
	if [[ "$(id -u)" -ne 0 ]]; then
		echo "error: must run as root (vppctl socket, ip netns exec need it)" >&2
		exit 1
	fi
}

require_vppctl() {
	if [[ -z "$VPPCTL_BIN" ]]; then
		echo "error: VPPCTL_BIN is required — point it at the vppctl from the same" >&2
		echo "       build as the running vpp, e.g.:" >&2
		echo "  sudo env VPPCTL_BIN=\"\$HOME/vpp/build-root/install-vpp-native/vpp/bin/vppctl\" \\" >&2
		echo "      $0 <label> -- <load-generator command...>" >&2
		exit 1
	fi
	if [[ "$VPPCTL_BIN" == */* ]]; then
		[[ -x "$VPPCTL_BIN" ]] || {
			echo "error: VPPCTL_BIN is not executable: $VPPCTL_BIN" >&2
			exit 1
		}
	else
		command -v "$VPPCTL_BIN" >/dev/null || {
			echo "error: VPPCTL_BIN not found in PATH: $VPPCTL_BIN" >&2
			exit 1
		}
	fi
}

# Fail fast when vppctl can't talk to VPP — losing 0 seconds beats losing a
# full load-gen run followed by empty snapshots. Exit code 139 (SIGSEGV)
# here is the signature of a vppctl-vs-vpp binary-API mismatch, which is
# exactly why VPPCTL_BIN is required above.
check_vpp_reachable() {
	local out rc=0
	out="$(vppctl show version 2>&1)" || rc=$?
	if ((rc != 0)); then
		echo "error: vppctl exited ${rc} talking to VPP on ${VPPCTL_SOCK}" >&2
		if [[ -n "$out" ]]; then
			echo "vppctl output: ${out}" >&2
		fi
		if ((rc == 139)); then
			echo "hint: SIGSEGV from vppctl usually means binary-API mismatch —" >&2
			echo "      VPPCTL_BIN must be the vppctl from the same build as the running vpp" >&2
		else
			echo "hint: is vpp running, and does VPPCTL_SOCK match 'cli-listen' in its .conf?" >&2
		fi
		exit 1
	fi
}

main() {
	require_root

	[[ $# -ge 1 ]] || usage
	local label=$1
	shift

	[[ "${1:-}" == "--" ]] || usage
	shift
	[[ $# -ge 1 ]] || usage

	# Label becomes a directory name — keep it filesystem-safe.
	[[ "$label" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || usage

	require_vppctl
	check_vpp_reachable

	local out_dir="${RESULTS_ROOT}/${label}"
	mkdir -p "$out_dir"

	vppctl clear run
	vppctl clear errors
	vppctl clear hardware-interfaces

	# Deliberately outside set -e: a failure (connection refused, crash
	# mid-run, non-zero exit) is exactly the moment you want VPP's counters
	# from — the snapshot must survive it, and the status must still reach
	# the caller via the exit code below.
	set +e
	"$@" 2>&1 | tee "${out_dir}/loadgen_output.txt"
	local load_status=${PIPESTATUS[0]}

	# Post-run vppctl calls stay non-fatal for the same reason: if VPP
	# itself died mid-run, these files are the post-mortem, and a failing
	# vppctl here must not hide load_status or the remaining files.
	vppctl show run >"${out_dir}/show_run.txt"
	vppctl show errors >"${out_dir}/show_errors.txt"
	vppctl show hardware-interfaces >"${out_dir}/show_hardware_interfaces.txt"
	set -e

	if [[ $load_status -ne 0 ]]; then
		echo "load-generator failed (exit ${load_status}); snapshot of the failure saved: ${out_dir}" >&2
		exit "$load_status"
	fi

	echo "snapshot saved: ${out_dir}"
}

main "$@"
