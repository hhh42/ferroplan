#!/bin/sh
# Fetch and build VAL (the external plan validator) into benchmarks/.val/.
# After this, benchmarks/run.py and benchmarks/ipc67.py pick it up via
# $FERROPLAN_VAL, e.g.:
#   sh benchmarks/get-val.sh
#   FERROPLAN_VAL=benchmarks/.val/VAL/build/bin/Validate python3 benchmarks/run.py
set -e
cd "$(dirname "$0")"
mkdir -p .val && cd .val
[ -d VAL ] || git clone --depth 1 https://github.com/KCL-Planning/VAL
cd VAL && mkdir -p build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release >/dev/null
make -j"$(nproc 2>/dev/null || echo 4)" Validate
echo "built: $(pwd)/bin/Validate"
