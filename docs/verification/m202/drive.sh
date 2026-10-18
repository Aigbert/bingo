#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/bingo"
OUTPUT=$(mktemp -d)
WORK=$(mktemp -d)
SOCKET=m202-drive
echo "Drive output: $OUTPUT"
if tmux -L "$SOCKET" list-sessions >/dev/null 2>&1; then
  echo "$SOCKET is already in use" >&2
  rm -rf "$WORK"
  exit 1
fi
cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT
pane() { tmux -L "$SOCKET" capture-pane -p -t drive; }
await_text() {
  for ((i=0; i<400; i++)); do
    if pane | grep -qE "$1"; then return; fi
    sleep 0.025
  done
  pane >&2
  echo "missing: $1" >&2
  exit 1
}
absent() {
  if pane | grep -qE "$1"; then pane >&2; echo "unexpected: $1" >&2; exit 1; fi
}
save() {
  pane | sed -E "s#${WORK##*/}#<drive>#g; s#session [A-Z0-9]+#session <id>#g" > "$OUTPUT/$1-${width}x${height}.txt"
}
for size in '100 32' '80 24'; do
  read -r width height <<< "$size"
  mkdir -p "$WORK/$width/home/.bingo" "$WORK/$width/data" "$WORK/$width/config" "$WORK/$width/cwd"
  printf '%s\n' '{"update":{"check":false}}' > "$WORK/$width/home/.bingo/settings.json"
  printf '%s\n' 'one' 'two' 'three' > "$WORK/$width/cwd/note.txt"
  cat > "$WORK/$width/script.json" <<'JSON'
{"responses":[
 {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"note.txt"}}},{"toolCall":{"name":"Edit","input":{"file_path":"note.txt","old_string":"two","new_string":"TWO"}}},{"toolCall":{"name":"Bash","input":{"command":"sleep 2 && cat note.txt"}}}]},
 {"steps":[{"text":"The reached-call drive is done."}]},
 {"steps":[{"toolCall":{"name":"Edit","input":{"file_path":"note.txt","old_string":"TWO","new_string":"two"}}},{"toolCall":{"name":"Bash","input":{"command":"echo never"}}}]}
]}
JSON
  HOME="$WORK/$width/home" XDG_DATA_HOME="$WORK/$width/data" tmux -L "$SOCKET" -f /dev/null new-session -d -s drive -x "$width" -y "$height" -c "$WORK/$width/cwd" \
    "env -u OPENAI_API_KEY -u OPENAI_BASE_URL -u ANTHROPIC_API_KEY -u ANTHROPIC_BASE_URL HOME='$WORK/$width/home' XDG_DATA_HOME='$WORK/$width/data' XDG_CONFIG_HOME='$WORK/$width/config' BINGO_MOTION=on BINGO_FAKE_SCRIPT='$WORK/$width/script.json' '$BIN' --cwd '$WORK/$width/cwd' --allowed-tools 'Bash,Read'"
  tmux -L "$SOCKET" pipe-pane -o -t drive "cat > '$OUTPUT/terminal-${width}x${height}'"
  await_text '/ commands'
  tmux -L "$SOCKET" send-keys -t drive -l 'read and change note.txt, then show it'
  tmux -L "$SOCKET" send-keys -t drive Enter
  await_text 'read +note.txt +3 lines'
  await_text 'edit +note.txt.*needs you'
  absent 'run +sleep 2'
  save asking
  if pane | grep -qF 'esc back to the ask'; then
    tmux -L "$SOCKET" send-keys -t drive -l "$(printf '\033[27u')"
  fi
  # A keyboard approval must pass the card's accidental-key guard.
  sleep 0.5
  tmux -L "$SOCKET" send-keys -t drive 1
  await_text '[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] run +sleep 2'
  save running
  await_text 'The reached-call drive is done\.'
  await_text '\$ run +sleep 2'
  await_text 'Reply, or / for commands'
  save done
  turn_two_offset=$(wc -c < "$OUTPUT/terminal-${width}x${height}")
  tmux -L "$SOCKET" send-keys -t drive -l 'stop the second edit'
  tmux -L "$SOCKET" send-keys -t drive Enter
  await_text 'edit +note.txt.*needs you'
  absent 'echo never'
  save asking-again
  tmux -L "$SOCKET" send-keys -t drive C-c
  await_text 'edit +note.txt.*stopped'
  await_text 'stop the second edit'
  absent 'echo never'
  save stopped
  python3 - "$OUTPUT/terminal-${width}x${height}" "$turn_two_offset" <<'PY'
import re, sys
raw = open(sys.argv[1], 'rb').read()[int(sys.argv[2]):].decode('utf-8', 'replace')
plain = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', raw)
assert 'echo never' not in plain, 'unreached command reached the terminal'
PY
  tmux -L "$SOCKET" kill-server
  echo "${width}x${height}: read landed, edit asked, later call hidden, allowed once, bash ran, second edit stopped passed"
done
