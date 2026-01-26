#!/bin/bash
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/cosmos_server" "$@"

