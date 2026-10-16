#!/usr/bin/env bash
# M185 hands-on drive: a scripted turn through a private tmux, captured at one
# size. Usage: drive.sh <cols> <rows> <outdir>
set -euo pipefail
unset OPENAI_API_KEY OPENAI_BASE_URL ANTHROPIC_API_KEY ANTHROPIC_BASE_URL

COLS=$1 ROWS=$2 OUT=$3
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
BIN=$ROOT/target/debug/bingo
SOCKET=m185-drive-$COLS
SESSION=drive
TRIES=300
WORK=$(mktemp -d)
cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT
mkdir -p "$WORK/home/.bingo" "$WORK/cwd/src" "$OUT"
printf '{ "update": { "check": false } }' >"$WORK/home/.bingo/settings.json"
printf 'fn main() {\n    println!("{}", total());\n}\n' >"$WORK/cwd/src/main.rs"

pane() { tmux -L "$SOCKET" capture-pane -p -t "$SESSION"; }
keys() { tmux -L "$SOCKET" send-keys -t "$SESSION" "$@"; }
await() {
  local i=0
  while [ "$i" -lt "$TRIES" ]; do
    if pane | grep -qF -- "$1"; then return 0; fi
    sleep 0.05
    i=$((i + 1))
  done
  echo "drive: timed out waiting for: $1" >&2
  pane >&2
  return 1
}
shot() { pane >"$OUT/$1_${COLS}x${ROWS}.txt"; }
esc() { tmux -L "$SOCKET" send-keys -t "$SESSION" -l "$(printf '\033[27u')"; }
# Answer the open card yes, pressing until `$1` shows; a key goes only to
# a card that holds the focus, then the composer is cleared of strays.
answer() {
  await 'Do you want to'
  local i=0
  while ! pane | grep -qF -- "$1"; do
    [ "$i" -lt "$TRIES" ] || { echo "drive: y never gave: $1" >&2; pane >&2; return 1; }
    if pane | grep -qF 'esc back to the ask'; then esc
    elif pane | grep -qF -- '⏎ answer'; then keys y
    fi
    sleep 0.05
    i=$((i + 1))
  done
  keys C-u
}

cat >"$WORK/script.json" <<'EOF'
{"responses":[
 {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"src/main.rs"}}}]},
 {"steps":[{"toolCall":{"name":"Grep","input":{"pattern":"total"}}}]},
 {"steps":[{"text":"One place prints it."},{"toolCall":{"name":"Edit","input":{"file_path":"src/main.rs","old_string":"\"{}\"","new_string":"\"total: {}\""}}}]},
 {"steps":[{"toolCall":{"name":"Bash","input":{"command":"python3 -c 'import time; time.sleep(4)'; echo 3 passed"}}}]},
 {"steps":[{"text":"Done. main now prints a labelled total."}]},
 {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"src/main.rs"}}}]},
 {"steps":[{"toolCall":{"name":"Grep","input":{"pattern":"total"}}}]},
 {"steps":[{"toolCall":{"name":"Bash","input":{"command":"python3 -c 'import time; time.sleep(60)'"}}}]},
 {"steps":[{"text":"too late"}]}
]}
EOF

HOME="$WORK/home" XDG_DATA_HOME="$WORK/data" XDG_CONFIG_HOME="$WORK/config" SHELL=/bin/sh \
  tmux -L "$SOCKET" -f /dev/null new-session -d -s "$SESSION" -c "$WORK/cwd" -x "$COLS" -y "$ROWS" \
  "PS1='drive\$ ' /bin/sh -i"
keys "HOME=$WORK/home XDG_DATA_HOME=$WORK/data BINGO_FAKE_SCRIPT=$WORK/script.json $BIN --cwd $WORK/cwd" Enter
await '/ commands'

keys 'label the total in main' Enter
await 'edit src/main.rs'
sleep 0.3
shot asked
answer 'bash python3'
answer 'esc stop'
sleep 1
shot running
await 'Done. main now prints'
sleep 0.5
shot finished
keys C-o
sleep 0.5
shot expanded
# A second ctrl+o goes on to the pager; esc comes back.
keys C-o
sleep 0.5
shot pager
esc
sleep 0.5

keys 'again, but stop' Enter
# The sleep runs without asking when an earlier yes was for the session.
i=0
until pane | grep -qF 'b background'; do
  if pane | grep -qF 'Do you want to'; then answer 'b background'; break; fi
  [ "$i" -lt "$TRIES" ] || { echo "drive: the sleep never ran" >&2; pane >&2; exit 1; }
  sleep 0.05
  i=$((i + 1))
done
sleep 1
esc
await 'you stopped the turn'
sleep 0.5
shot stopped
keys C-o
sleep 0.5
shot stopped_expanded
