#!/usr/bin/env bash
# Run with bash; never overwrite existing VPP files or build as root.
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
vpp_dir="$(realpath -- "${1:?Usage: bash scripts/prepare-vpp.sh /path/to/vpp}")"
[[ -f "$vpp_dir/src/cmake/plugin.cmake" && -d "$vpp_dir/test" ]] || {
  echo "Expected a VPP source checkout: $vpp_dir" >&2
  exit 1
}
link_once() {
  local source_path="$1" destination_path="$2"
  if [[ -L "$destination_path" ]] && [[ "$(readlink -f -- "$destination_path")" == "$source_path" ]]; then
    return
  fi
  if [[ -e "$destination_path" || -L "$destination_path" ]]; then
    echo "Refusing to overwrite: $destination_path" >&2
    exit 1
  fi
  ln -s -- "$source_path" "$destination_path"
}
link_once "$repo_dir/plugin" "$vpp_dir/src/plugins/rust_classify"
link_once "$repo_dir/tests/test_rust_classify.py" "$vpp_dir/test/test_rust_classify.py"
echo "Linked plugin and test into $vpp_dir. Build and run commands: docs/wsl.md"
