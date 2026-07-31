#!/bin/sh
# Run the bounded OStar MuStar/SigmaStar adapter.
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 "$here/openai_ostar_star.py"
