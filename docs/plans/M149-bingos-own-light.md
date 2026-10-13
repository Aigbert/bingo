# M149 — bingo's own light

## Goal

Implement [tui-identity-2026-10.md](../design/tui-identity-2026-10.md) L1–L8:
caret, ember amber, token-driven embers, wordmark, own verbs and code bands.
Keep cards, keys, the esc stack, tool bullets and result connectors.
TUI-only; no new dependency, sdk or wire change. M150 remains out of scope.

## Re-verification (2026-10-14, `ed98a22f`)

M108–M136, M147 and M148 are present; M125/M148 dependencies are satisfied.
The tree started clean on `m149-bingos-own-light`; review commit `f7d62df1`.
`render::Render` scopes the look; `Focus::Intro(Playing)` owns the opening;
`layout/deadline.rs::next` collects activity deadlines; `view.rs` draws the
foot on the air row. `blocks.rs` composes the welcome. Markdown shares
`views/code.rs`. ADRs 0001, 0002, 0070 and 0081 hold; none needs accepting.
The L7 boundary stop (`9154e672`) was resolved by the user: defer the earned
variant (`f90cc226`); keep the generic worked row with its caret.

## Bricks, in build order

1. Failing TUI contracts first: dark/light palette, Unicode/ASCII marks,
   530 ms waiting blink, streaming glow, 2.2 s blocked breath, dim stopping;
   64-token crossings, four embers maximum, 0.6 s lifetime; wordmark frames
   at 0.0/0.5/1.2/2.0/2.4 s; code language shares the raised band.
2. Change only presence/glow: dark `#f0883e/#ffc278`, light `#a84d12/#e07a2a`.
3. Replace sparkle with `▍`; thoughts use `▏`; ASCII uses `|` for both.
   Update the window title and dim worked row; retain its existing words.
4. Derive the caret style from the turn phase and clock. Keep the input
   border's breath and the transcript's comet.
5. Derive ember crossings from arriving assistant/reasoning text: one token
   per four characters is an animation estimate, since reported usage
   arrives only after a round. Usage counters keep reported tokens. Hold
   only the last four birth instants per turn; clear at its end. The foot
   outranks embers; motion-off draws none; the air row's height is unchanged.
6. Assistant blocks use the caret: bold text at rest, presence while streaming.
7. Replace the box with the site's half-block wordmark, facts, a reserved
   resume row and M63's update row. Read the newest earlier root in this cwd
   on the host-call lane. Under 40 columns/16 rows use `bingo▍`; ASCII `bingo|`.
   Type a column per 16 ms, blink twice, throw three embers, rest at 2.4 s.
8. Use Kindling, Stoking, Smouldering, Foraging, Tracking, Burrowing,
   Sniffing around, Pouncing, Sifting and Tinkering.
9. Code uses its own raised width, with a dim language at the first row's
   right edge; keep numbering past eight lines and inline backticks.
10. Update tui.md §§1,4,6,10,11 and the identity record; review snapshots
    and drive the owned tmux socket at all required sizes and looks.

## Files

- TUI `theme`, `activity`, `embers`, `identity`, `transcript`, `worked`,
  `welcome`, `opening`, `run`, `markdown`, `motion`, `lib`, `ui`, `view`,
  `blocks`, `layout`, `render`, `paths`; `opening/*`, `run/opening.rs`,
  `transcript/thinking.rs`, `views/code.rs`, `layout/deadline.rs`.
- TUI snapshots/tests (`screens/*`, `painted`, `motion/*`, `layout/tests`);
  mechanical glyph expectations in `select`, `search`, `transcript/pictured`.
- `crates/bingo/tests/pty/**` only for removed-glyph needles;
  `scripts/tui-smoke.sh` changes the ASCII answer needle to `| Hello in ascii.`.
- `docs/design/tui.md`, `tui-identity-2026-10.md`; terminal capture evidence
  in `docs/design/m149-tmux-captures.txt` (commit `9f319d19`, removed after merge).
- M137's host/runtime, plugin-rpc, manifests and schemas are excluded.

## Exit criteria

- [x] Brick-1 contracts failed first and now pass; TestBackend pins the air
      row; tests pin foot priority, motion-off and compact-height behaviour.
- [x] Removed glyphs, colours, greeting and Simmering have no matches in crates.
- [x] 189 snapshots reviewed; overlapping families: welcome 177, activity 32,
      worked 0 (assertions), thinking 18, model block 58, code 9, palette 1;
      other families 0.
- [x] tmux: 120×40, 80×24, 38×14; dark/light OSC 11, NO_COLOR, ASCII,
      motion-off; opening, streaming/embers, tool, stop and worked captures.
- [x] TUI smoke and all native protocol gates.
- [ ] TUI Windows check: left to Windows CI under protocol §7; `aws-lc-sys`
      cannot find Windows `stdlib.h`/`windows.h` on this Mac.

## Non-goals

- L7's earned `Bingo · N tasks done in …` variant: it needs a producer-owned
  completion contract, which is a later milestone's work (user, 2026-10-14).
- M150's agent faces/crew/chat and M151's pane placement.
- The site's tokens/hero and GUI live in other repositories; report the
  two accent pairs for their owners to carry over. Inline backticks stay.

## Risks

- Row count stays fixed during each opening and ember animation; removing
  the old box/language row intentionally changes their resting content.
- Ambiguous-width carets/half-blocks retain the existing terminal constraint;
  the ASCII look uses none of them.
- The first workspace run hit the existing gateway claim test under parallel
  tests. It passed alone; the complete suite is rerun with one test thread.
- The streaming estimate is for motion only, not billed token accounting.
- Wordmark frame snapshots retain trailing cells to pin animation width.
- Captures were committed in `9f319d19` (`docs/design/m149-tmux-captures.txt`) and removed after merge.
  No live/credential tests, sdk/wire/schema changes or new dependencies.

## Verified (2026-10-14)

```text
$ cargo test -p bingo-surface-tui --locked identity:: -- --nocapture
Before: test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 1102 filtered out; finished in 0.13s
After: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1079 filtered out; finished in 0.14s
$ cargo test -p bingo-surface-tui --locked embers::tests::
Before: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1106 filtered out; finished in 0.02s
After: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1084 filtered out; finished in 0.00s
$ cargo fmt --all -- --check
exit 0
$ cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.45s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.23s
$ cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed
exit 0; aggregate: 5153 passed, 0 failed, 4 ignored; all 16 PTY tests passed
$ cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.04s
$ cargo test -p bingo-provider-acp --test bridge --locked token -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
$ cargo test -p bingo-gateway --test claim --locked doctor_never_removes_the_gateway_claim_file -- --exact --test-threads=1
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.01s
$ scripts/check_discipline.sh
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 355 (max 355)
warm cargo check -p bingo-core: 1s (max 20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 5 GB (soft max 5)
test binaries: 102
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p bingo-surface-tui --all-targets --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
fatal error: 'stdlib.h' file not found
fatal error: 'windows.h' file not found
$ scripts/tui-smoke.sh
tui-smoke ok
$ python3 /tmp/m149-drive.py
M149 tmux: 15 configurations captured
$ rg -c '^[0-9]+x[0-9]+ [a-z-]+:' docs/design/m149-tmux-captures.txt
90
$ rg '✻|✢|✶|✽|d97757|f2a07c|Welcome to bingo|Simmering' crates/ --stats
0 matches
$ git diff --check ed98a22f -- . ':!**/bingo_surface_tui__identity__wordmark_*.snap'
exit 0
```
