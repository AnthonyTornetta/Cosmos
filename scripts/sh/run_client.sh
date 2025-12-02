#!/bin/bash
HERE="$(dirname "$(readlink -f "$0")")"
# Allows unix systems to find the steam library
export LD_LIBRARY_PATH="$HERE:$LD_LIBRARY_PATH"
exec "$HERE/cosmos_client" "$@"

