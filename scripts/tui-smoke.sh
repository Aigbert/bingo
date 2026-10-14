#!/usr/bin/env bash
# The terminal surface through a real pty. tmux gives us the terminal a unit
# test cannot: a reply arrives, Esc interrupts a running turn, a permission
# dialog answered `y` really writes the file, and ctrl+c twice hands the
# terminal back with status 0.
#
# Every wait is a bounded poll of `capture-pane`, never a fixed sleep: the
# script is as fast as the binary and fails loudly instead of flaking.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/bingo"
SESSION=smoke
# 200 polls of 50 ms: ten seconds is far more than any step needs.
TRIES=200

command -v tmux >/dev/null || { echo "tui-smoke: tmux is required"; exit 1; }
[ -x "$BIN" ] || { echo "tui-smoke: build first (cargo build): $BIN"; exit 1; }

WORK=$(mktemp -d)
SOCKET="bingo-smoke-${WORK##*/}"
cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/home/.bingo" "$WORK/cwd"
# The smoke reaches nothing outward. A run with a terminal asks once a day
# whether a newer release is out (M63); this one says no.
printf '{ "update": { "check": false } }' >"$WORK/home/.bingo/settings.json"

pane() { tmux -L "$SOCKET" capture-pane -p -t "$SESSION"; }
keys() { tmux -L "$SOCKET" send-keys -t "$SESSION" "$@"; }

# Wait for `$1` to appear on the pane, or fail with what was there instead.
await() {
  local needle="$1" i=0
  while [ "$i" -lt "$TRIES" ]; do
    if pane | grep -qF -- "$needle"; then return 0; fi
    sleep 0.05
    i=$((i + 1))
  done
  echo "tui-smoke: timed out waiting for: $needle" >&2
  echo "--- pane ---" >&2
  pane >&2
  return 1
}

step() { printf '  %s\n' "$1"; }

# Wait for `$1` to leave the pane. Motion takes frames, so this polls like
# `await` rather than looking once.
vanish() {
  local needle="$1" i=0
  while [ "$i" -lt "$TRIES" ]; do
    if ! pane | grep -qF -- "$needle"; then return 0; fi
    sleep 0.05
    i=$((i + 1))
  done
  echo "tui-smoke: it never left: $needle" >&2
  echo "--- pane ---" >&2
  pane >&2
  return 1
}

# A reply of eighty numbered rows: more transcript than the screen holds.
# They are list items, so markdown keeps them one to a row.
long_reply() {
  local text="" i
  for i in $(seq 1 80); do text="$text- line $i\\n"; done
  printf '{"responses":[{"steps":[{"text":"%s"}]}]}' "$text"
}

# What a terminal sends when its window takes the focus or loses it, once
# focus reporting is on: `\033[I` and `\033[O`.
focus() {
  tmux -L "$SOCKET" send-keys -t "$SESSION" -l "$(printf '\033[%s' "$1")"
}

# One SGR mouse report: button, column, row (1-based, as a terminal sends it).
mouse() {
  tmux -L "$SOCKET" send-keys -t "$SESSION" -l "$(printf '\033[<%s;%s;%sM' "$1" "$2" "$3")"
}

# A real picture on disk, since a picture reaches a transcript by being read.
# Two blue pixels: the smallest thing the fs tool will call `image/png`.
SHOT=iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEElEQVR42mO4WR4ORAwQCgAwWgadFBKCuwAAAABJRU5ErkJggg==
picture() { printf '%s' "$SHOT" | base64 -d >"$WORK/cwd/shot.png"; }

# Start bingo on a scripted provider. The script's own text never appears in
# the command line, so `await` matches the output and not the echo of itself.
# `$2` is any extra environment the step wants, `$3` any extra flags.
start() {
  printf '%s' "$1" >"$WORK/script.json"
  keys "HOME=$WORK/home ${2:-} BINGO_FAKE_SCRIPT=$WORK/script.json $BIN --cwd $WORK/cwd ${3:-}" Enter
  await '? for shortcuts'
}

# Press `$1` until `$2` shows. A dialog ignores keys until its guard has
# passed (the kernel's, not ours), so the first press may land on nothing.
press_until() {
  local i=0
  while ! pane | grep -qF -- "$2"; do
    if [ "$i" -ge "$TRIES" ]; then
      echo "tui-smoke: pressing '$1' never produced: $2" >&2
      echo "--- pane ---" >&2
      pane >&2
      return 1
    fi
    keys "$1"
    sleep 0.05
    i=$((i + 1))
  done
}

# Leave, and prove the shell has the terminal back with the status we expect.
# Wait until the shell holds the pane again. A key typed while bingo is
# still leaving goes to bingo, not to the shell, and is never seen again.
gone() {
  local i=0
  while [ "$(tmux -L "$SOCKET" display-message -p -t "$SESSION" '#{pane_current_command}')" = bingo ]; do
    if [ "$i" -ge "$TRIES" ]; then
      echo "tui-smoke: bingo never left the pane" >&2
      echo "--- pane ---" >&2
      pane >&2
      return 1
    fi
    sleep 0.05
    i=$((i + 1))
  done
}

# Each leaving is numbered: the pane keeps the last one's line, and a wait
# for the same words would match it before this shell has even come back.
LEFT=0
finish() {
  # A retry loop may have left a stray key in the composer, and a draft
  # takes the hint off the status line.
  keys C-u
  # While a turn runs the status line says `esc to interrupt`, and a ctrl+c
  # then interrupts the turn instead of asking to leave: wait for it to end.
  await "${1:-? for shortcuts}"
  keys C-c
  await 'press ctrl+c again to exit'
  keys C-c
  gone
  LEFT=$((LEFT + 1))
  keys "echo \"left $LEFT with: \$?\"" Enter
  await "left $LEFT with: 0"
}

tmux -L "$SOCKET" -f /dev/null new-session -d -s "$SESSION" -x 120 -y 40 \
  "PS1='smoke\$ ' /bin/sh -i"
keys 'echo "shell: $((1 + 1))"' Enter
await 'shell: 2'

step 'a reply reaches the transcript'
start '{"responses":[{"steps":[{"text":"Hello from the smoke test."}]}]}'
keys 'say hello' Enter
await 'Hello from the smoke test.'
if pane | grep -Eq '# (Tasks|Experience|Memories)|Context injection|context:memory'; then
  echo 'tui-smoke: internal context appeared in the conversation' >&2
  pane >&2
  exit 1
fi
finish

step 'esc interrupts a turn that is still waiting'
start '{"responses":[{"steps":[{"delay":{"ms":60000}},{"text":"too late"}]}]}'
keys 'wait for it' Enter
await 'esc to interrupt'
keys Escape
await '[Request interrupted by user]'
finish

# The same keypress before a single byte has come back: the request is still
# being established, which is where a long context spends its seconds.
step 'esc interrupts a turn whose request has not answered yet'
start '{"responses":[{"steps":[{"delayBeforeStream":{"ms":60000}},{"text":"too late"}]}]}'
keys 'wait for the first byte' Enter
await 'esc to interrupt'
keys Escape
await '[Request interrupted by user]'
finish

step 'a page up releases the tail, and the foot takes it back'
start "$(long_reply)"
keys 'say a lot' Enter
await ' line 80'
keys PPage
await ' line 20'
vanish ' line 80'
keys NPage
await ' line 80'
finish

step 'the wheel scrolls the transcript'
start "$(long_reply)"
keys 'say a lot' Enter
await ' line 80'
for _ in 1 2 3 4 5 6 7 8 9 10; do mouse 64 10 5; done
vanish ' line 80'
finish

step 'ctrl+f searches the transcript and esc gives the status line back'
start "$(long_reply)"
keys 'say a lot' Enter
await ' line 80'
keys C-f
keys 'line 42'
await '/line 42'
keys Enter
await '1/1 · n/N · esc'
keys Escape
await '? for shortcuts'
finish

step 'a focused block opens whole in the pager and gives the frame back'
start "$(long_reply)"
keys 'say a lot' Enter
await ' line 80'
# A click takes the block; the frame it is answered against is the last one
# drawn, so the drive waits for it before pressing anything.
mouse 0 10 5
sleep 0.5
keys Enter
await 'j/k · pgup/pgdn'
keys 'G'
await ' line 80'
keys '/'
keys 'line 42'
await '/line 42'
keys Enter
await 'n/N · esc'
keys Escape
keys Escape
vanish 'j/k · pgup/pgdn'
await '? for shortcuts'
finish

step 'an @ offers the paths under the session and enter takes one'
printf '[package]\nname = "smoke"\n' >"$WORK/cwd/Cargo.toml"
start '{"responses":[{"steps":[{"text":"nothing to do"}]}]}'
keys '@Car'
await '@Cargo.toml'
keys Enter
await '> @Cargo.toml'
finish

step 'the help sheet opens on ? and closes on esc'
start '{"responses":[{"steps":[{"text":"nothing to do"}]}]}'
keys '?'
await 'shift+tab'
keys Escape
await '? for shortcuts'
vanish 'shift+tab'
finish

# The fixture has one session, so `↓` on an empty box rightly keeps its old
# job (there is nowhere to walk to) and `ctrl+g` is the door that always opens.
# What this drives is the layer, not the walk: a real terminal draws it, keys
# reach it, and `esc` gives the frame back.
step 'ctrl+g opens the one list of sessions and esc closes it'
start '{"responses":[{"steps":[{"text":"nothing to do"}]}]}'
keys 'ask me' Enter
await 'nothing to do'
keys C-g
await '❯ ◗  ask me  idle · completed'
keys Escape
vanish '❯ ◗  ask me'
await '? for shortcuts'
finish

step 'a permission dialog answered y runs the tool'
start '{"responses":[
  {"steps":[{"toolCall":{"name":"Write","input":{"file_path":"note.txt","content":"written by the smoke test\n"}}}]},
  {"steps":[{"text":"Wrote it."}]}]}'
keys 'write the note' Enter
await 'Do you want to '
# The dialog is a card: a bordered box, not rows in the transcript.
await '│'
press_until 'y' 'Wrote it.'
[ -f "$WORK/cwd/note.txt" ] || { echo "tui-smoke: the approved Write wrote nothing" >&2; exit 1; }
grep -q 'written by the smoke test' "$WORK/cwd/note.txt"
finish

step 'a notice holds the status line and then leaves it'
start '{"responses":[{"steps":[{"text":"nothing to do"}]}]}'
keys C-c
await 'press ctrl+c again to exit'
# It holds its window and goes on its own: nothing is pressed to dismiss it.
vanish 'press ctrl+c again to exit'
await '? for shortcuts'
finish

step 'a question on a window nobody watches notifies and nothing else'
start '{"responses":[
  {"steps":[{"toolCall":{"name":"Write","input":{"file_path":"away.txt","content":"written while away\n"}}}]},
  {"steps":[{"text":"Wrote it."}]}]}'
# The window loses the focus, so the question that follows goes to the desktop
# as well as to the screen — and not one byte of it may land on the pane.
focus O
keys 'write the note' Enter
await 'Do you want to '
press_until 'y' 'Wrote it.'
[ -f "$WORK/cwd/away.txt" ] || { echo "tui-smoke: the approved Write wrote nothing" >&2; exit 1; }
if pane | grep -qE 'notify|777|Ptmux'; then
  echo 'tui-smoke: a notification was printed onto the screen' >&2
  pane >&2
  exit 1
fi
focus I
finish

# tmux is a terminal of its own, so the graphics probe is not asked through it
# (M46) and every picture here takes design §5's degrade. That is the half of
# the row this script can prove on a real terminal: the words are right and no
# escape of a protocol the terminal never agreed to reaches the screen. The
# other half — a terminal that answers the probe and is sent the picture — is
# driven by `crates/bingo/tests/pty/`, which can answer it.
step 'a picture is named where the terminal does not draw pictures'
picture
start '{"responses":[
  {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"shot.png"}}}]},
  {"steps":[{"text":"That is the picture."}]}]}'
keys 'look at it' Enter
await '[image: image/png]'
await 'That is the picture.'
if pane | grep -qE '_Ga=|10eeee'; then
  echo 'tui-smoke: a graphics sequence reached a terminal that was never asked' >&2
  pane >&2
  exit 1
fi
finish

step 'BINGO_GRAPHICS=off asks nothing and still names the picture'
picture
start '{"responses":[
  {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"shot.png"}}}]},
  {"steps":[{"text":"That is the picture."}]}]}' 'BINGO_GRAPHICS=off'
keys 'look at it' Enter
await '[image: image/png]'
finish

step 'BINGO_ASCII=1 and NO_COLOR leave a terminal nothing it cannot draw'
start '{"responses":[{"steps":[{"text":"Hello in ascii."}]}]}' 'BINGO_ASCII=1 NO_COLOR=1'
keys 'say hello' Enter
await '| Hello in ascii.'
if pane | grep -q '⏺'; then
  echo 'tui-smoke: a glyph outside the ascii table survived BINGO_ASCII=1' >&2
  pane >&2
  exit 1
fi
finish

for size in '80 24' '120 40' '200 50'; do
  read -r width height <<<"$size"
  tmux -L "$SOCKET" resize-window -t "$SESSION" -x "$width" -y "$height"
  step "placed board and activity at ${width}x${height} with a room open"
  start '{"responses":[
    {"steps":[{"toolCall":{"name":"DemoProgress","input":{"label":"cargo test"}}}]},
    {"steps":[{"text":"The bar has run."}]}]}' 'BINGO_MOTION=off' '--demo-ui'
  keys '/room design' Enter
  await '#design'
  keys '/board' Enter
  await '[ 1 Tick ]'
  keys Tab
  await '❯ Board'
  keys '1'
  await 'running'
  # The removed shortcut neither opens a sheet nor takes focus from the row.
  keys C-p
  keys '2'
  await 'board reset'
  await '❯ Board'
  keys Tab
  keys 'run the bar' Enter
  await '0 %'
  await '100 %'
  await 'The bar has run.'
  finish
  step "${width}x${height}: board Tick/Reset and Activity ok; ctrl+p unbound"
done

mkdir -p "$WORK/home/.bingo/plugins/counter"
cp "$ROOT/crates/bingo/tests/fixtures/mods/counter/"{plugin.json,main.ts} "$WORK/home/.bingo/plugins/counter/"
for width in 80 200; do
  tmux -L "$SOCKET" resize-window -t "$SESSION" -x "$width" -y 24
  step "pulled counter at ${width}x24"
  start '{"responses":[{"steps":[{"text":"Ready."}]}]}' 'BINGO_MOTION=off'
  keys '/counter' Enter
  await 'Count: 0'
  keys 'i'
  await 'Count: 1'
  keys Tab
  keys 'Ada'
  await 'Name: Ada'
  keys Escape
  vanish 'Count: 1'
  keys '/counter' Enter
  await 'Count: 1'
  await 'Name: Ada'
  keys Escape
  vanish 'Count: 1'
  finish
  step "${width}x24: Counter 0→1; Input Ada persisted; Escape closed"
done

step 'M141 tool-call, band and pane examples'
for example in tool-call band pane; do
  mkdir -p "$WORK/$example/types"
  cp "$ROOT/examples/plugins/$example/"{plugin.json,register.ts,README.md} "$WORK/$example/"
  cp "$ROOT/examples/plugins/$example/types/index.d.ts" "$WORK/$example/types/"
done
printf 'original' >"$WORK/cwd/protected.txt"
start '{"responses":[
  {"steps":[{"toolCall":{"name":"Edit","input":{"file_path":"protected.txt","old_string":"original","new_string":"changed"}}}]},
  {"steps":[{"toolCall":{"name":"Bash","input":{"command":"echo before"}}}]},
  {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"protected.txt"}}}]},
  {"steps":[{"text":"All done."}]}]}' 'BINGO_MOTION=off' "--plugin-dir $WORK/tool-call --plugin-dir $WORK/band --plugin-dir $WORK/pane --allowed-tools 'Edit,Bash(echo:*),Read'"
await 'A small band above your prompt'
keys M-h
vanish 'Hide band'
keys '/ledger' Enter
await 'No calls yet'
keys Escape
vanish 'Tool ledger'
keys 'Run the example calls' Enter
await 'All done.'
await 'Example calls: 2'
[ "$(cat "$WORK/cwd/protected.txt")" = original ]
keys '/ledger' Enter
await 'Tool ledger'
await 'Bash ·'
await 'Read ·'
keys Escape
vanish 'Tool ledger'
finish 'Example calls: 2'
step 'M141: band hidden; Edit denied; Bash rewritten; two pane rows; Escape closed'

echo 'tui-smoke ok'
