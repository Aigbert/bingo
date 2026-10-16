#!/bin/bash
# usage: pr101-drive.sh <workdir> <bingo binary>
W=$1; B=$2
rm -rf "$W/home" "$W/data" "$W/out"; mkdir -p "$W/home" "$W/data" "$W/cwd" "$W/out"
echo '{"responses":[{"steps":[{"text":"ok"}]}]}' > "$W/script.json"
T="tmux -L pr101 -f /dev/null"
$T kill-server 2>/dev/null
$T new-session -d -s a -x 120 -y 40 "env -u OPENAI_API_KEY -u OPENAI_BASE_URL -u ANTHROPIC_API_KEY -u ANTHROPIC_BASE_URL HOME=$W/home XDG_DATA_HOME=$W/data BINGO_MOTION=off BINGO_FAKE_SCRIPT=$W/script.json $B --cwd $W/cwd"
sleep 5
cap(){ $T capture-pane -p -t a > "$W/out/$1.txt"; }
typ(){ $T send-keys -t a -l "$1"; sleep 0.4; }
key(){ $T send-keys -t a "$1"; sleep "${2:-1}"; }
typ "/"; sleep 0.6; cap 01-bare-slash; key Escape 0.5
typ "/age"; sleep 0.6; cap 02-search-age; key Escape 0.5
typ "/agents"; key Enter 1.5; cap 03-agents; key Escape 0.6
typ "/room lab"; key Enter 2; cap 04-room-open
typ "/crew rooms"; key Enter 1.5; cap 05-crew-rooms
key Enter 1.5; cap 06-room-view
key Escape 1.5; cap 07-after-esc
key Escape 0.5
typ "/rewind"; key Enter 1.5; cap 08-rewind; key Escape 0.6; key Escape 0.4
typ "/mcp"; key Enter 1.5; cap 09-mcp; key Escape 0.6
typ "/permission"; key Enter 1.2; cap 10-permission; key Escape 0.6
typ "/crew team"; key Enter 1.5; cap 11-crew-team; key Escape 0.6
typ "/agents stop nobody"; key Enter 1.5; cap 12-agents-verb
$T kill-server
