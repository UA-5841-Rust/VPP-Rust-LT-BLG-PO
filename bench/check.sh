#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
for script in bench/*.sh; do bash -n "$script"; done
python3 -m py_compile bench/*.py
echo 'All bench Bash/Python syntax checks passed.'
