# M123 — Slots II: the TUI names no plugin

## Goal

Implement ADR-0070 §§4–5,7 and ADR-0071 §4: producers stamp voices and place rows;
the TUI reads navigation through `bingo-collab` and names no plugin or tool.
No new host verb or external dependency.

## Bricks, in build order

1. Contract first: golden `Voice`, unchanged legacy `Origin`, optional
   `Row.caption` wire/schema tests, stranger rendering and collab room fixtures.
2. Add optional `Origin.voice`, omitted when absent, and `Origin::voiced`.
3. Stamp producers beside their records: agents message, bash notification,
   schedule runner and command prompt are `Quiet`; wake is `Note`; room
   nudge is `Aside`; channels and agents' submit hook are explicitly `Said`.
   Context pieces remain unstamped; M122's serial bounce is a tool result.
4. TUI `said::voice` reads the stamp, then the frozen `LEGACY_VOICES` table,
   else `Said`; pending activity uses the same reader. Remove old constants.
5. `HookContext.item` is populated for before/after tool hooks from the
   gated/outcome item (including M131 `chain/tool.rs`); otherwise `None`.
6. Memory's before-tool hook filters `Read`/`Write`/`Edit`, reads its own
   directories, and extends `row:<item>` with its verb/title and memory badge.
   Keep directories for the prompt/guide; move naming rules and tests here.
7. Both skill doors publish `Row { mark: '❖', title: "Skill", badge: name,
   caption: args, body: None }`: tool execution and the turn-start hook for
   its own `/name` prompt. Caption defaults to absent and is omitted when
   absent on wire. `Row.body` keeps replacement semantics (user, 2026-10-13).
8. Bash jobs place foreground rows with `background`, `bash.promote`, the
   call id and key `b`. Ctrl+B fires the newest unfinished item's placed
   action; remove the TUI's promote/tool-name lookup, keep `NOTHING_RUNNING`.
9. Move room record parsing into `bingo-collab` readers. TUI seats, roster
   and mentions consume library types/constants, retaining their drawing.
10. Discipline 4f rejects quoted plugin ids and 4c tool names in production
    TUI code; exclude cfg(test) modules, its manifest id and ADR-0076 `acp.rs`.
    Seed violations to prove failure. Keep all screen snapshots unchanged.
11. Record ownership in design/tui.md §§4,8; accept ADR-0070 §§4–5,7 and
    ADR-0071 §4, with dated amendments to ADR-0034 §3 and ADR-0044.

## Files

- `crates/bingo-sdk/src/{event.rs,hook.rs,slot.rs}`, `fixtures/`; generated `schema/{plugin,rpc}.json`
- `crates/bingo-core/src/{gate.rs,turn.rs,chain/tool.rs,session.rs,session/commands.rs,session/invoke.rs}`
- `crates/bingo-context/src/{lib.rs,memory/mod.rs}`, a new `memory/row.rs`
- `crates/bingo-skills/src/{lib.rs,tool.rs}`, a new `hook.rs`
- `crates/bingo-tool-bash/src/{jobs,promote,notify,lib}.rs`
- `crates/bingo-agents/src/{message,hook}.rs`, `crates/bingo-rooms/src/{post,serial}.rs`,
  `crates/bingo-schedule/src/{wake,runner}.rs`, `crates/bingo-channels/src/runner.rs`
- `crates/bingo-collab/src/room.rs` and `room/readers.rs` (readers)
- `crates/bingo-surface-tui/{Cargo.toml,src/{transcript,transcript/said,transcript/slots,pager,blocks,input,seats,roster,mentions,activity,theme}.rs}`;
  `{memory,skill}.rs` deleted; `screens.rs`, `screens/skills.rs`, `screens/colours.rs`, `test_support.rs`, `lib.rs`
- `Cargo.lock` (TUI library dependency); mechanical `Origin`/`HookContext`
  constructors in core, checkpoints, provider-acp, hooks-shell, plugin-rpc,
  agents, rooms, context, schedule and bin tests; `test_lanes.rs` adds only absent captions.
- `scripts/check_discipline.sh`, `docs/design/tui.md`,
  `docs/adr/{0034,0044,0070,0071}-*.md`

## Exit criteria

- [x] `cargo test -p bingo-sdk voice` passes; the failing first run is pasted.
- [x] `rg '"_?bingo\.' crates/bingo-surface-tui/src --glob '!**/tests/**'` finds nothing in production except the manifest id and the ADR-0076 ACP exception; check 4f rejects seeded plugin/tool literals (output pasted).
- [x] `rg 'QUIET_SURFACES|ROOMS_READING|PROMOTE|fn running_shell' crates/bingo-surface-tui/src` finds nothing; only `LEGACY_VOICES` retains old surface words.
- [x] Each producer asserts its voice or its `Row` beside the record; the stranger test draws a `Row`'s title, badge and action.
- [x] All `screens/` snapshots are unchanged; the diff is empty.
- [x] `scripts/tui-smoke.sh` passes, and a tmux drive shows a memory row, `/guide` and a model `Skill` call, ctrl+b on a running Bash call, a room's switcher columns and `@` completion.
- [x] All gates and Windows cross-checks attempted; C-header-only exceptions remain for Windows CI under protocol step 7 (listed below).


## Non-goals

- `acp.rs`'s tool names, which ADR-0076 removes.
- Print and channels drawing voices or rows; they keep their own rules.
- Any change to what rooms or memory record.

## Risks

- Resolved 2026-10-13 by the user: add optional `Row.caption`, defaulted
  and omitted when absent. Draw it after the badge on the heading;
  `Row.body` keeps replacement semantics. Keep `test_lanes.rs:201` unchanged.
- Contract first: pin rows with/without caption and its optional schema;
  regenerate plugin/RPC schemas only through the repository generator.
- Resumed at `dde18848`; preserved the prior exit-criterion ticks. No further Decision change.
- Re-verified at `e415e97e`: M131 dispatch is in `chain/tool.rs`; M122's
  bounce is a tool result and its holder constant is `name::PARENT`.

## Verified (2026-10-13)

Earlier contract-first output, retained from the interrupted run:
```text
cargo test -p bingo-sdk voice                         # first run
E0432: unresolved import bingo_sdk::Voice
E0599: no method named voiced; E0609: no field voice
cargo test -p bingo-sdk voice                         # implemented
2 passed; 0 failed (origin fixture unchanged)
cargo test -p bingo-core before_deny_ends_the_run      # first: E0609 cx.item
1 passed; 0 failed (implemented)
cargo test -p bingo-collab room_readers                # first: missing readers
1 passed; 0 failed (implemented)
cargo test -p bingo-surface-tui a_stranger_places      # pre-caption experiment
FAILED: !screen.contains("old body") at test_lanes.rs:201
cargo test -p bingo-surface-tui a_stranger_places --locked # final renderer
2 passed; 0 failed (original body-replacement assertion retained)
```
- Producer run: `cargo test -p bingo-context -p bingo-skills -p bingo-tool-bash -p bingo-agents -p bingo-rooms -p bingo-schedule -p bingo-channels -p bingo-core --locked`.
  Agents `155 passed`, channels `104`, context `99`, core `463`, rooms `230`, schedule `115` (+1 integration).
  Skills first: `83 passed; 3 failed` (`this double extends nothing`); its tool-test host now accepts rows.
  `cargo test -p bingo-skills -p bingo-tool-bash --locked`: `86 passed` / `147 passed; 0 failed`.
  Explicit complete memory/skill/promotion rows and every origin voice are asserted beside their producers.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc the_committed_schema_is_this_document --locked`:
  each `1 passed; 0 failed`; schemas regenerated, never hand-edited. M132's merger may regenerate conflicts.
- Discipline 4f, extracted unchanged: `Unseeded: 0 TUI names no plugin or tool`;
  each seed (`bingo.stranger`, `_bingo.stranger`, `Bash`): `exit 1; TUI names a plugin or tool`.
  Seeds removed. Removed-symbol `rg`: exit 1, no matches. Snapshot diff against dev: `0` lines.
- `PATH=/tmp/m123-verification/bin:$PATH scripts/tui-smoke.sh`: `tui-smoke ok` (private socket shim).
  `python3 /tmp/m123-verification/drive.py`: `M123 tmux drive ok`; captured output:
```text
⏺ Recall from memory(the-build)
❖ Skill(guide) the interface                # /guide
❖ Skill(guide) the wire                     # model Skill call
⏺ Bash(sleep 45; echo promotion-complete)    # [ b background ] before Ctrl+B
  ⎿  Running in the background              # 1 shell after Ctrl+B
~ scout     idle · in #design · live
⏺ reviewer idle · in #design · listening · 600s
#design  2 seats
❯ @all
  @scout
  @reviewer
m123-exit:0
```
  The drive corrected its Bash-description expectation and created standby agents for seat columns;
  no TUI change. Owned tmux only; temporary homes/journals removed. Logs/captures: `/tmp/m123-verification/`.
- Sequential gates (logs in that directory):
  `cargo fmt --all -- --check`: `fmt exit: 0`;
  `cargo check --workspace --all-targets --locked`: ``Finished `dev` profile ... in 5.75s``;
  `cargo clippy --workspace --all-targets --locked -- -D warnings`: ``Finished `dev` profile ... in 4.18s``.
- `cargo test --workspace --locked -- --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_may_be_dialled_again_once_its_stream_has_closed`:
  summed `5133 passed; 0 failed; 4 ignored` (existing measurement, stress and two storyboard tests); PTY `16 passed`, no retry.
  `cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1`:
  `1 passed; 0 failed`; `cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1`:
  `1 passed; 0 failed`. Neither known parallel-hang test was silently skipped.
- `scripts/check_discipline.sh`: `discipline ok`; `scripts/budget.sh`: `dependencies: 352 (max 352)`,
  `warm cargo check: 0s`, `relink isolation: 0`, `target/debug: 7 GB (soft max 5)`, `budget ok`.
  `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- Windows, one crate at a time: `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`.
  sdk/core/collab/context/skills/tool-bash/agents/rooms/schedule/checkpoints/hooks-shell/provider-acp/surface-rpc:
  each ``Finished `dev` profile``, exit 0. Names above have the `bingo-` prefix.
  channels/plugin-rpc/surface-tui: `failed to run custom build command for aws-lc-sys v0.44.0`;
  bingo: same plus `tree-sitter v0.26.13` / `tree-sitter-bash v0.25.1`.
  `fatal error: 'stdlib.h' file not found` (also `stdio.h`/`windows.h`): protocol step 7 leaves these four
  checks to Windows CI. No Rust compile error was reported; no toolchain installed. No other criterion remains open.
