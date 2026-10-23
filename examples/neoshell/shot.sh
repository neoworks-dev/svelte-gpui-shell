#!/usr/bin/env bash
# Runs the built shell on the headless output `neoshell-port` and saves a screenshot.
#
#   examples/neoshell/shot.sh OUT.png [SECONDS] [host flags…]
#
# Everything that could reach the user's session is pointed elsewhere: surfaces open
# only on the headless output, the config and control socket live under /tmp and
# $XDG_RUNTIME_DIR/neoshell-port, and the shell refuses every bus function that changes
# the system (compositor dispatch, volume, launching, trash, …). Set NEOSHELL_ALLOW to
# let one through for a test against scratch paths. NEOSHELL_CONFIG names another config.
#
# SHELL_BUNDLE names another bundle, such as dist/smoke.js.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out=$1
seconds=${2:-4}
[ $# -gt 0 ] && shift
[ $# -gt 0 ] && shift

scratch=/tmp/neoshell-port
mkdir -p "$scratch" "$XDG_RUNTIME_DIR/neoshell-port"
if [ ! -f "$scratch/config.json" ]; then echo '{}' >"$scratch/config.json"; fi

export OUTPUT=neoshell-port
export NEOSHELL_CONFIG=${NEOSHELL_CONFIG:-$scratch/config.json}
export NEOSHELL_SOCKET=$XDG_RUNTIME_DIR/neoshell-port/host.sock
export NEOSHELL_MONITORS=neoshell-port
export NEOSHELL_READONLY=1
exec "$root/scripts/shot.sh" "${SHELL_BUNDLE:-$here/dist/neoshell.js}" "$out" "$seconds" --dev "$@"
