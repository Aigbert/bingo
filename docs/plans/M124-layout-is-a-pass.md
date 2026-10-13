# M124 — Layout is a pass

## Goal

Audit-2026-10 §4: replace layout as a drawing side effect with
`layout(previous, tree, ui, size, now) -> Layout`, then `paint(&Layout, frame)`.
One render per band, one screen memo, and an earliest `next_frame: Option<Instant>`
for the loop. No visible change, new dependency
or SDK change. ADR-0002, ADR-0013 and ADR-0070 remain the contracts.

## Bricks, in build order

1. Contract tests first: equal inputs give equal layouts; each band renders
   at most once and unchanged blocks are reused; rail, picture and foot hits
   need no terminal; idle, motion, notice, exit and countdown deadlines.
2. `Layout` owns regions, composed visible cells and cursor, the block memo,
   height/top, card/list/rail/strip/picture targets and foot. Picture cells
   come from the laid-out lines (`graphics::placed::cells`, M56).
3. Consume the previous memo. Measure composer, strip and activity from the
   lines this pass already rendered. Patch the foot into the reserved air
   after measuring the transcript. Keep only screen-sized composed cells
   beside the existing block memo; exercise a 10,000-item transcript.
4. Paint copies cells and cursor; it reads no tree, UI or clock. Delete
   `Painted::begin`, `RefCell<Painted>` and writes through a shared UI borrow.
5. Publish `Ui.shown` after painting. Input, pointer, pager, selection and
   graphics delivery read that plain layout. Move picture and line mappings.
6. Each cue supplies its next change: continuous motion keeps its 33 ms
   samples; layers/progress use steps; notices, exit and countdowns use their
   deadlines. Include scroll, crossfade, send, blocks, intro and drag.
   Delete `Run::animating`; `behind` still owes a frame at the draw gate.
7. Record layout/paint in tui.md §3, deadlines in §6, and the dated
   screen-memo note in ADR-0002.

## Files

Under `crates/bingo-surface-tui/src/`:
- `layout.rs`, `layout/{canvas,count,deadline,tests}.rs` (new), `frame.rs`;
- `{view,ui,run,input,pointer,pager,blocks,select}.rs`, `run/showing.rs`;
- `{activity,rail,status,composer,foot,scroll,layers,search,clock}.rs`,
  `composer/strip.rs`, `views/{mod,progress}.rs`;
- `{test_support,screens,motion,painted,doubles,terminal,lib}.rs`,
  `transcript/pictured.rs`, `graphics/{placed,decoded}.rs`;
- mechanical render-helper fixtures: `screens/`, `motion/landing.rs`,
  `test_lanes.rs`, `transcript.rs`, `transcript/slots.rs`, `worked.rs`;
- `test_support/rendered.rs` extracts the drawing fixtures below the file limit.
Records: `docs/design/tui.md`, `docs/adr/0002-*.md`, this plan.

## Exit criteria

- [x] Layout contract tests pass; first failing output is pasted below.
- [x] `rg 'ui::Painted|RefCell<Painted>|painted\.borrow|fn animating' crates/bingo-surface-tui/src` finds nothing.
- [x] Pointer tests click a rail card and picture without a terminal draw.
- [x] Counters prove one render per band and none for unchanged blocks.
- [x] An injected idle minute draws nothing; running motion owes each sample.
- [x] Screen/motion assertions and snapshots pass; list every changed snapshot.
- [x] PTY smoke and owned tmux drive: scroll, rail click, picture click,
      drag-select, resize, idle minute without redraw.
- [x] Protocol gates and the TUI MSVC cross-check are recorded.

## Non-goals

M125 focus/render-context globals; changed region content or geometry;
block revision keys; M148/M149 appearance. `frame::RAIL_AT` and
`frame::RAIL_WIDTH` remain named constants for M148's later thresholds.

## Risks and re-verification

Re-verified at `abe1755f`, after M108–M123, before implementation:
- 61 `painted.borrow` sites: input 21, view 19 (10 mutable), pointer 9,
  run 4, pager 2, run/showing 2, transcript/pictured 2, ui 2.
- `Painted` has ten fields including `regions`; `animating` has 13
  conditions, including `behind`, intro and placed status countdowns.
- `Revision` has nine keys: status, size, agent, fold, linked, slots,
  run, last, look. M123 captions, `LEGACY_VOICES` and check 4f stay intact.
- The screen adapter and fixture helpers also need the explicit layout
  handoff. Their assertions stay; mutability changes are mechanical.
- A glyph changes every 150 ms, but breathing already samples every 33 ms.
  Keep that cadence: forcing extra wall-clock glyph boundaries changed the
  flash test's frame count. M149's caret and embers can add next-change cues.
- Prepare directory completion before layout; image header measurements
  move with its consumed memo. Theme/graphics globals remain M125's scope.
- A drag reads the layout on screen. A slow draw must still show its final
  motion frame; a notice created after painting must mark another frame owed.
- No snapshot movement is authorized. No parallel agent's files are edited.

## Verified (2026-10-13)

Contract commits preceded implementation. Trimmed command output:
```text
$ cargo test -p bingo-surface-tui layout --locked  # first run
error[E0432]: unresolved imports `super::Layout`, `super::layout`
$ cargo test -p bingo-surface-tui layout --locked  # implemented
 test result: ok. 15 passed; 0 failed
$ cargo test -p bingo-surface-tui --locked a_notice_created_after_painting_owes_a_frame
 test result: FAILED. 0 passed; 1 failed  # next_draw was None
 test result: ok. 1 passed; 0 failed     # after marking a frame owed
```
The added progress contract first panicked at `expect("moving rail")`; it now passes.
The glyph/breath contract was corrected
to preserve the existing 33 ms sampling, as noted above.

Gates ran sequentially. The known ACP token tests were excluded only from
the workspace invocation with two `--skip` arguments, then run alone:
```text
$ cargo fmt --all -- --check
# exit 0, no output
$ cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.89s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.82s
$ INSTA_UPDATE=no cargo test --workspace --locked -- --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_may_be_dialled_again_once_its_stream_has_closed
# aggregate: 5167 passed; 0 failed; 4 ignored; 2 filtered out
 test result: ok. 1132 passed; 0 failed; 2 ignored  # TUI
 test result: ok. 16 passed; 0 failed              # PTY, no retry
$ cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
 test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
$ cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1
 test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out
$ scripts/check_discipline.sh
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 352 (max  352)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ TMUX_TMPDIR=<own scratch directory> scripts/tui-smoke.sh
tui-smoke ok
$ cargo check -p bingo-surface-tui --all-targets --locked --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
fatal error: 'stdlib.h' file not found
fatal error: 'windows.h' file not found
```

TUI MSVC is deferred to Windows CI under protocol step 7 (C headers only; no toolchain installed).
The first discipline run found `test_support.rs: 1003 non-test lines
(>1000)`; extracting its drawing helpers resolved the in-scope failure.
Changed snapshots: **none** (`git diff --name-only abe1755f -- '*.snap'`
is empty). The removal search in Exit criteria returns no matches.
```text
$ python3 /tmp/m124-drive.py  # owned tmux + PTY, isolated home, fake provider
wheel scroll and foot passed
rail card click passed
picture click reached disabled opener
drag selection and copy passed
resize 120x40 -> 100x30 -> 120x40 passed
idle minute: 0 bytes, no redraw
tmux drive passed
```
