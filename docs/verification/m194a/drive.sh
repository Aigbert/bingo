#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BIN="$ROOT/target/debug/bingo"
WORK=$(mktemp -d)
SOCKET="m194-drive-${WORK##*/}"
cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT
pane() { tmux -L "$SOCKET" capture-pane -p -t drive; }
await_text() {
  for ((i=0; i<400; i++)); do
    if pane | grep -qF "$1"; then return; fi
    sleep 0.05
  done
  pane >&2
  echo "missing: $1" >&2
  exit 1
}
await_idle() {
  for ((i=0; i<400; i++)); do
    if ! pane | grep -qF 'esc stop'; then return; fi
    sleep 0.05
  done
  echo 'turn did not finish' >&2
  exit 1
}
for size in '100 32' '80 24'; do
  read -r width height <<< "$size"
  mkdir -p "$WORK/$width/home/.bingo" "$WORK/$width/data" "$WORK/$width/config" "$WORK/$width/cwd"
  printf '%s\n' '{"update":{"check":false}}' > "$WORK/$width/home/.bingo/settings.json"
  printf '%s\n' 'a file for the look drive' > "$WORK/$width/cwd/note.txt"
  cat > "$WORK/$width/script.json" <<'JSON'
{"responses":[
 {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"note.txt"}}}]},
 {"steps":[{"delay":{"ms":3000}},{"text":"The look drive is done."}]},
 {"steps":[{"delay":{"ms":60000}},{"text":"This turn should be stopped."}]}
]}
JSON
  HOME="$WORK/$width/home" XDG_DATA_HOME="$WORK/$width/data" tmux -L "$SOCKET" -f /dev/null new-session -d -s drive -x "$width" -y "$height" -c "$WORK/$width/cwd" \
    "env -u OPENAI_API_KEY -u OPENAI_BASE_URL -u ANTHROPIC_API_KEY -u ANTHROPIC_BASE_URL HOME='$WORK/$width/home' XDG_DATA_HOME='$WORK/$width/data' XDG_CONFIG_HOME='$WORK/$width/config' BINGO_FAKE_SCRIPT='$WORK/$width/script.json' '$BIN' --cwd '$WORK/$width/cwd'"
  await_text '/ commands'
  tmux -L "$SOCKET" send-keys -t drive -l 'read note.txt'
  tmux -L "$SOCKET" send-keys -t drive Enter
  await_text 'esc stop'
  await_text '✓ read note.txt'
  pane > "$ROOT/docs/verification/m194a/live-${width}x${height}.txt"
  await_text 'The look drive is done.'
  await_idle
  pane > "$ROOT/docs/verification/m194a/done-${width}x${height}.txt"
  tmux -L "$SOCKET" send-keys -t drive -l 'wait until I stop'
  tmux -L "$SOCKET" send-keys -t drive Enter
  await_text 'esc stop'
  tmux -L "$SOCKET" send-keys -t drive Escape
  await_text 'you stopped the turn'
  pane > "$ROOT/docs/verification/m194a/stopped-${width}x${height}.txt"
  tmux -L "$SOCKET" kill-server
  echo "${width}x${height}: live, done, stopped passed"
done
