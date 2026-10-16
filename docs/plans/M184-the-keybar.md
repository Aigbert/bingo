# M184 — The KeyBar

ADR-0095 §1, §2, §8; ADR-0084 §1, §4. One right-aligned row that says what the
keys do now, and the place toasts appear.

## Goal

No pane draws key hints in its body; every state's pairs appear in one row,
`key` in bold glow, the action dim, ` · ` in ash, matching the prototype's
`_keys.txt` per state. Toasts sit on its left for 2.8 s and nothing moves.

## Bricks

1. **sdk props**: `KeyBar { pairs, toast }`; a pane's key pairs as props on its
   seat (`Slot::Pane` hints). Fixture tests first.
2. **Built-in owner** `bingo-mod-keybar` draws the row from the focused site's
   pairs and the keymap; sealed while an ask is open (ADR-0084 §4): the host
   draws the ask's pairs.
3. **Mods declare pairs** instead of drawing them: models, palette, status,
   plugins, sessions, browse, crew, memory, schedules, ask.
4. **Composer tag** (`? `, `⟲ `, `/model › sign in`, …) and placeholder as
   `Composer` props; the stray `/model` echo under a pane goes.
5. **Toasts** move into the bar (one at a time, wall clock, dropped on expiry);
   pending other sessions and repo waiting become pairs (`ctrl+g 2 need you`).
6. **Narrow widths** drop pairs from the left; the toast wins over pairs.

## Files

- `crates/bingo-sdk` (component props), new `crates/bingo-mod-keybar`.
- `crates/bingo-surface-tui/src/{frame.rs,view.rs,toast.rs,keys.rs,sealed.rs}`.
- Each mod's `draw.rs` that drew hints.

The built-in adds one workspace member (421 → 422), no new external package.
The existing `unicode-width` measures whole pairs.
Pulled trees declare `SiteHints` on their root Box; pushed panes may declare
them on their seat. Both use the same SDK contract, never body text parsing.

## Exit criteria

- [x] Snapshot per state from the prototype's `_keys.txt` list.
- [x] A test per mod: no hint text inside its body.
- [x] tmux drive of five states with captures in Verified.
- [x] Gates as in M182: native gates pass; Windows TUI cross-check remains blocked.

## Non-goals

Changing what any key does. Keymap remapping UI.

## Risks

- Sealed ask pairs must stay the host's: a mod's pairs never draw over an open ask.
- Width churn: the bar's pairs change with focus; the row height never changes.

## Verified

2026-10-16, macOS arm64. Provider credentials/base URLs unset for all commands;
interactive runs used fake, temporary HOME/cwd and private `tmux -L m184-* -f /dev/null`.
Cargo and Node ran directly, bypassing mise wrappers for isolated-home tests.

- Reference rendering: 85 states at 108/80/32 columns in `crates/bingo-surface-tui/src/keybar/snapshots/`; inputs in `prototype.json` beside them. The private `_keys.txt` was read only.
- Owner tests cover all ten mods; TestBackend covers styles, narrowing, fixed geometry, replacement, sealed asks, rebinding and expiry without replay.
- Tmux captures: `docs/verification/m184/01-idle.txt` through `05-permission.txt`; `06-mode-no-notice.txt` verifies bypass → accept edits without a stale notice.
- Existing bindings take precedence over prototype labels: provider fields remain Enter-to-edit; reason/change Escape stops, ctrl+g returns; question text Escape skips. Permission-mode notices are deliberately absent.

```text
exit 0 — cargo fmt --all -- --check: no output
exit 0 — cargo check --workspace --all-targets --locked: Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.02s
exit 0 — cargo clippy --workspace --all-targets --locked -- -D warnings: Finished `dev` profile in 3.70s
exit 0 — RUST_TEST_THREADS=4 cargo test --workspace --locked --no-fail-fast: 116 suites, 6153 passed, 0 failed, 0 ignored
exit 0 — RUST_TEST_THREADS=4 cargo test -p bingo-surface-tui --lib --locked: 1315 passed; 0 failed
exit 0 — scripts/check_discipline.sh: discipline ok
exit 0 — scripts/budget.sh: 422/422 dependencies; warm core 0s/20s; isolation 0; budget ok
exit 0 — cargo deny check: advisories ok, bans ok, licenses ok, sources ok
exit 0 — TUI_SMOKE_SOCKET_PREFIX=m184-smoke scripts/tui-smoke.sh: tui-smoke ok
exit 0 — private tmux drive: five states captured, plus bypass → accept edits regression
exit 0 — cargo check -p bingo-sdk -p bingo-mod-keybar -p bingo-mod-models -p bingo-mod-palette --all-targets --locked --target x86_64-pc-windows-msvc: Finished `dev` profile
exit 0 — cargo clean (then --target-dir for this worktree’s resolved target): Removed 177896 files, 35.0GiB total
exit 101 — cargo check -p bingo-surface-tui --all-targets --locked --target x86_64-pc-windows-msvc: aws-lc-sys cannot find stdlib.h/windows.h
```

Windows TUI compilation is unverified: this Mac has no MSVC C headers/toolchain
(the same dependency blocker recorded in M182/M183). No push or merge was made.
Discipline's data-driven snapshot-name warnings are non-fatal; the budget reported
a 30 GB target directory above its 5 GB soft limit before the required cleanup.
Cleanup removed both the target symlink and its resolved build directory.

Rebase verified — 2026-10-16: onto dev `c957120a`. Conflicts: palette `draw.rs`
kept dev's held-key args (no `key_props`) and M184's root `hints`; pty picture tests
now live in `pictures.rs` (dev), their wait string moved to `/ commands`. Dev's
eighty-column notice test moved to `keybar/tests.rs`; `HERDR_OFF` reaches the KeyBar
as a 2.8 s toast. Review fixes: one `KeyBarProps::fitting` brick for host and mod;
toast keys no longer placed are forgotten again; the busy box is empty as M183 says;
the smoke waits for the login pane, since the palette's KeyBar also says `↑↓ pick`.
Private `tmux -L m184v`, temp HOME/cwd, fake provider, 120×40: idle, `/` palette,
`/model ` args, `/status` pane and the permission ask (sealed: `↑↓ pick · ⏎ answer ·
esc no, stop the turn`) all one KeyBar row above the footer; 12 sent Up/Down keys
settled 16–17 ms after the last one.

```text
exit 0 — cargo fmt --all -- --check
exit 0 — cargo clippy --workspace --all-targets --locked -- -D warnings
exit 0 — RUST_TEST_THREADS=4 cargo test --workspace --locked --no-fail-fast: 116 suites, 6191 passed, 0 failed
exit 0 — scripts/check_discipline.sh: discipline ok (after renaming `room`, a kernel noun)
exit 0 — scripts/budget.sh: 422/422 dependencies; warm core 1s/20s; budget ok
exit 0 — cargo deny check: advisories ok, bans ok, licenses ok, sources ok
exit 0 — TUI_SMOKE_SOCKET_PREFIX=m184v-smoke scripts/tui-smoke.sh: tui-smoke ok, 4 of 4
exit 0 — cargo check -p bingo-sdk -p bingo-mod-keybar --all-targets --locked --target x86_64-pc-windows-msvc
```
