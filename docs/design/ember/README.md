# bingo Ember — the terminal's spec

The person approved this prototype whole on 2026-10-18 (ADR-0097). It is the spec for what the
terminal draws, cell by cell (ADR-0095 as amended); plans M192–M200 build it.

- [`SPEC.md`](SPEC.md) — the short form: bands, grid, a turn, the edge line, acting, crew views,
  line breaking, inks, marks, tool marks, the motion budget, the scenes.
- [`prototype/`](prototype/) — the prototype's source. `build.sh` assembles
  `target/ember/bingo-ember.html`; open it in a browser to play every scene with the keyboard,
  in light and dark, ASCII, reduced motion, 100×32 and 80×24. Its renderer is the reference:
  where this README, SPEC.md or a plan disagree with it, the prototype wins.
- [`scenes/`](scenes/) — 105 captures exported from the prototype by `prototype/capture.js`.

## The captures

| File | What it holds |
|---|---|
| `<scene>.txt` | The scene's still at the moment the gallery shows it: a header line, then H rows of W cells. A wide character fills two cells and is written once. |
| `<scene>-<state>.txt` | The same scene after the keys and waits that `index.json` lists under `after`. |
| `ascii-*`, `reduced-*`, `small-*` | Scenes in ASCII, under reduced motion, at 80×24. |
| `<name>.ink` | Each row's inks: `bg` runs, then `fg` runs, as `x0-x1 token[+b][+i][+u]` (columns from 0, inclusive). A token is a name from SPEC.md's Ink (`fg`, `muted`, `dim`, `you`, `well`, `glow`, `crew0`…, and `kw`/`fn`/`str`… for code); a hex value is a blend caught mid-animation. |
| `index.json` | Every capture: scene, group, moment in ms, size, flags, and the keys before it. |

Each capture runs the prototype's own engine on a private clock: 50 ms ticks, 60 ms after
each key. The same build always writes the same files.

## How a plan uses them

- Each scene a milestone owns gets a `TestBackend` test. The test builds the same state, using
  fixtures ported from the prototype's data in `prototype/p4b-data.js`, and renders at the
  capture's size and moment. Characters must match `<name>.txt` cell for cell. Inks must match
  `<name>.ink` wherever the capture names a token; where it holds a blend, only the character
  is compared.
- When a test and a capture differ, fix the code. If the prototype is the one that is wrong,
  change the prototype first: edit `prototype/`, rebuild, re-export, then commit the new
  captures together with the code that matches them. The person approves any change to what
  a scene shows.

## The shipped terminal follows them

Since M199 the shipped TUI draws every capture, not a sample. The sweep,
`crates/bingo-surface-tui/src/test_support/ember/sweep.rs`, names each of the 105 captures once
together with the milestone whose `full_frame` draws it, so a failing test is named for the
milestone that owns the regression. Each test renders the whole frame at the capture's size and
moment and compares it as above: characters cell for cell, inks wherever a token is named.
`every_capture_in_the_index_is_swept_once` reads `scenes/index.json` and fails when a capture is
missing from the sweep, swept twice, or swept without being in the index. A new capture is
therefore a failing test until a milestone draws it.

Run the sweep alone with `cargo test -p bingo-surface-tui --locked ember::sweep`.

## Regenerating

```sh
docs/design/ember/prototype/build.sh
PLAYWRIGHT_CORE=<path to playwright-core> CHROME=<chromium executable> \
  node docs/design/ember/prototype/capture.js target/ember/bingo-ember.html docs/design/ember/scenes
```

Node and playwright-core are a tool for this folder only. They are not a build or CI dependency.
