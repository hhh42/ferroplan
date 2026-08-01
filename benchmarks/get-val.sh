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
# VAL's CMakeLists declares a pre-3.5 minimum; CMake 4.x removed that
# compatibility outright, so configuring fails without the policy shim.
# Harmless on CMake 3.x (unknown -D vars are just cached).
cmake .. -DCMAKE_BUILD_TYPE=Release -DCMAKE_POLICY_VERSION_MINIMUM=3.5 >/dev/null
# nproc is GNU-only; macOS has sysctl. Fall back to 4 on neither.
make -j"$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)" Validate
echo "built: $(pwd)/bin/Validate"
