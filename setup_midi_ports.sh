#!/usr/bin/env bash
# Selects the host MIDI ports used by the MIDI tests and writes them to midi_ports.conf.
# Lists the ports seen by midi-tester and asks which host output is wired to the board's MIDI IN 1 and
# IN 2, and which host input to its MIDI OUT and THRU. The same port may be given several times.
# Stores the port names. Rerun to change.
# Usage: ./setup_midi_ports.sh
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONF="$REPO/midi_ports.conf"

cd "$REPO/midi-tester"
cargo build --release 2>&1
MT=target/release/midi-tester

# Prints the ports of one direction, asks for one and prints its name on stdout.
# $1: input|output   $2: what the port is wired to
choose() {
    local dir=$1 wired_to=$2
    local names=() line
    while IFS= read -r line; do
        names+=("${line#*: }")
    done < <("$MT" list --"$dir" | grep -E '^[0-9]+: ')
    if [[ ${#names[@]} -eq 0 ]]; then
        echo "No host MIDI $dir port found: connect the adapter and rerun" >&2
        exit 1
    fi

    echo "" >&2
    echo "Host MIDI $dir ports ($wired_to):" >&2
    local i
    for i in "${!names[@]}"; do
        echo "  $i: ${names[$i]}" >&2
    done
    local sel
    while true; do
        read -rp "Select the $dir port [0]: " sel
        sel=${sel:-0}
        [[ "$sel" =~ ^[0-9]+$ && $sel -lt ${#names[@]} ]] && break
        echo "Enter a number between 0 and $(( ${#names[@]} - 1 ))" >&2
    done
    printf '%s' "${names[$sel]}"
}

# Single-quotes the value for the config file, which the tests source
quote() {
    printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

IN1=$(choose output "wired to the board's MIDI IN 1")
IN2=$(choose output "wired to the board's MIDI IN 2")
OUT=$(choose input "wired to the board's MIDI OUT")
THRU=$(choose input "wired to the board's MIDI THRU")

cat > "$CONF" <<CONF_EOF
# Host MIDI port wired to each board connector, used by the MIDI tests. Written by ./setup_midi_ports.sh;
# rerun it or edit to change. Names as printed by \`midi-tester list\`; the same port may appear several times.
MIDI_IN1=$(quote "$IN1")    # host output -> board MIDI IN 1
MIDI_IN2=$(quote "$IN2")    # host output -> board MIDI IN 2
MIDI_OUT=$(quote "$OUT")    # board MIDI OUT -> host input
MIDI_THRU=$(quote "$THRU")  # board MIDI THRU -> host input
CONF_EOF

echo ""
echo "Wrote $CONF:"
cat "$CONF"
