#!/usr/bin/env bash
# Assemble the Ember prototype into one page: target/ember/bingo-ember.html under the repository.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/../../../../target/ember"
mkdir -p "$out"
{
  printf '<!doctype html><meta charset="utf-8">\n'
  cat "$here/p1-head.html" "$here/p2-body.html"
  printf "<script>(function(){'use strict';\n"
  for p in p3-core p4-model p4b-data p4c-act p4d-crew p5-stream p6-dock p7-input p8-page; do cat "$here/$p.js"; done
  printf '})();</script>\n'
} > "$out/bingo-ember.html"
echo "$(cd "$out" && pwd)/bingo-ember.html"
