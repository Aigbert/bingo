#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/bingo"
OUTPUT=${1:-$(mktemp -d)}
mkdir -p "$OUTPUT"
OUTPUT=$(cd "$OUTPUT" && pwd)
echo "Drive output: $OUTPUT"
WORK=$(mktemp -d)
SOCKET=m194b-drive
if tmux -L "$SOCKET" list-sessions >/dev/null 2>&1; then
  echo "m194b-drive is already in use" >&2
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
  if pane | grep -qE '▍ (working|thinking|burrowing)|▸ 1 step|you: yes|\[ b background \]|Worked for|is waiting for you'; then
    pane >&2; exit 1
  fi
}
no_finished_output() {
  if pane | grep -qE '^[[:space:]]*(one|two|three)[[:space:]]*$'; then
    pane >&2
    echo "a finished step still hangs its output" >&2
    exit 1
  fi
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
 {"steps":[{"delay":{"ms":1500}},{"toolCall":{"name":"Read","input":{"file_path":"note.txt"}}},{"toolCall":{"name":"Bash","input":{"command":"sleep 2 && cat note.txt"}}}]},
 {"steps":[{"delay":{"ms":1500}},{"text":"The step-row drive is done."}]},
 {"steps":[{"delay":{"ms":60000}},{"text":"This turn should be stopped."}]}
]}
JSON
  HOME="$WORK/$width/home" XDG_DATA_HOME="$WORK/$width/data" tmux -L "$SOCKET" -f /dev/null new-session -d -s drive -x "$width" -y "$height" -c "$WORK/$width/cwd" \
    "env -u OPENAI_API_KEY -u OPENAI_BASE_URL -u ANTHROPIC_API_KEY -u ANTHROPIC_BASE_URL HOME='$WORK/$width/home' XDG_DATA_HOME='$WORK/$width/data' XDG_CONFIG_HOME='$WORK/$width/config' BINGO_MOTION=on BINGO_FAKE_SCRIPT='$WORK/$width/script.json' '$BIN' --cwd '$WORK/$width/cwd' --allowed-tools 'Bash,Read'"
  tmux -L "$SOCKET" pipe-pane -o -t drive "cat > '$WORK/$width/terminal'"
  await_text '/ commands'
  tmux -L "$SOCKET" send-keys -t drive -l 'read note.txt and run the command'
  tmux -L "$SOCKET" send-keys -t drive Enter
  await_text 'working'
  await_text 'read note.txt and run the command.*1s'
  # The PTY can capture any phase of the declared 120 ms Spin beat.
  await_text '[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏] run +sleep 2'
  absent
  save live
  await_text 'read +note.txt +3 lines'
  await_text 'The step-row drive is done\.'
  await_text '\$ run +sleep 2'
  await_text 'Reply, or / for commands'
  absent
  no_finished_output
  save done
  tmux -L "$SOCKET" send-keys -t drive -l 'wait until I stop'
  tmux -L "$SOCKET" send-keys -t drive Enter
  await_text 'working'
  await_text 'wait until I stop.*1s'
  tmux -L "$SOCKET" send-keys -t drive -l "$(printf '\033[27u')"
  await_text 'wait until I stop.*stopped · [0-9]+s'
  # Cancellation can finish before capture-pane polls. The pipe retains the
  # acknowledged frame, including separately inked Shimmer characters.
  # The pipe consumer may flush after capture-pane sees the completed frame.
  python3 - "$WORK/$width/terminal" "$OUTPUT/terminal-failed-${width}x${height}.txt" <<'PY'
import re, sys, time
for _ in range(100):
    raw = open(sys.argv[1], 'rb').read().decode('utf-8', 'replace')
    plain = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', raw)
    if 'stopping' in plain:
        break
    time.sleep(.01)
else:
    open(sys.argv[2], 'w').write(raw)
    raise AssertionError('esc acknowledgement never reached the PTY')
PY
  if pane | grep -qE 'stopping|esc stop'; then pane >&2; exit 1; fi
  absent
  no_finished_output
  save stopped
  tmux -L "$SOCKET" kill-server
  echo "${width}x${height}: read upshot, clock, working, live, done, stopping, stopped passed"
done
