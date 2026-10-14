# M160 — The sealed sites

ADR-0082 §6.

## Goal

The permission prompt and the sign-in card become sites a mod may draw around,
never draw: the host's own drawing of who asks, which tool, the full input and
the choices is always on screen whole, or the mod's answer is not used.

## Bricks

1. **Fact props.** `Permission` from `InteractionKind` permission
   interactions: tool, input from the asking transcript item, the choices (`AnswerSpec`s) and the
   interaction id; `SignIn` from `InteractionKind::Login`: provider and the
   `LoginFlow` facts (browser url, device url and code, paste). No key names,
   no preformatted text. JSON fixture and round-trip test first.
2. **Sealed check, host side.** `Component::sealed()` is true for both. In
   `view::answer_ok` (core `chain/render.rs` path), a sealed component's
   answer must contain exactly one `Engine` node for the asked component, and
   its props must equal the request's props. Anything else is answered
   `Engine` alone, with the ADR-0079 §5 notice line naming the plugin. Pure
   function with table tests: zero nodes, two nodes, changed props, a
   different component's `Engine`, a wrapping `Box` that passes.
3. **Sealed check, surface side.** The TUI draws a sealed answer only when the
   `Engine` node's lines land inside the card whole: not clipped by the card's
   height, not cut by width truncation, not hidden by `display: none` or a
   zero-size box. Otherwise it draws its own card. Pure function on the laid
   out lines plus a marker for the engine node's span (`views::Marks` exists).
4. **Keys are the host's.** A sealed card's keys (y/n/a, enter, esc, paste)
   are dispatched by the TUI exactly as today, whatever the mod drew. Buttons
   a mod adds around the card fire their own actions and cannot answer the
   interaction.
5. **Text and desktop.** `surfaces()` lists `terminal`, `desktop`, `text`.
   The `text` surface (print, channels) keeps drawing folds; nothing changes
   there in this plan.

## Files

- `crates/bingo-sdk/src/{render.rs,component/props.rs,view/*}` (answer check).
- `crates/bingo-core/src/chain/render.rs`.
- `crates/bingo-surface-tui/src/{dialog.rs,view.rs,component.rs}` and a new
  small module for the whole-drawing check.
- `crates/bingo-mods/src/{guide.md,validate*}`; regenerated `schema/`,
  `sdk/typescript/`.
- Fixture mods: one that frames the permission card (accepted), one that
  rewrites its props, one that omits the engine node, one that clips it.

## Exit criteria

- [x] Fixture tests for `Permission` and `SignIn` props.
- [x] Table tests for the host check and the surface whole-drawing check.
- [x] Black-box: framing mod shows its frame and the host card; each hostile
      fixture yields the built-in card and one notice line; `y` still answers
      the interaction in every case.
- [x] No snapshot changes when no mod matches.
- [x] tmux drive in a harness-owned session: permission card with the framing
      mod at 80x24 and 200x50, answered with `y` and with `esc`.
- [x] Gates: fmt, check, clippy -D warnings, test, check_discipline, budget,
      cargo deny, `scripts/tui-smoke.sh`, Windows check (or recorded deferral).

## Non-goals

Mods answering interactions; changing permission policy or the order of
choices; channel (Feishu) permission cards; desktop drawing; new
dependencies.

## Risks

- A mod could imitate a permission card elsewhere (a pane drawn to look like
  one). Out of scope; recorded for a later look at pane chrome.
- Clipping detection must use the laid-out result, not the requested size, or
  a short terminal lets a clipped card through.
- Equality of props must be structural JSON equality after normalising both
  sides, so key order never fails an honest answer.

## Implementation findings

- `InteractionKind::Permission` carries a preview, not input. Project input
  from its referenced `ToolCall`; if it is absent or names a different tool,
  keep the built-in card. Do not duplicate input in the interaction record.
- The existing card abbreviates previews. A wrapped permission uses the host's
  full JSON input; unmatched cards retain their existing drawing. If it cannot
  fit, retain the built-in card and its expansion gesture.
- A surface receives a `View`, without plugin provenance. Core failures name
  the plugin through the existing notice path; layout failures show one local
  fallback notice, without inventing an attribution. Adding
  provenance to the render wire is outside this milestone.
- During the host's arrival animation, use the built-in card until it is whole.
  Mod buttons are pointer targets only; all interaction keys remain the host's.

## Verified — 2026-10-14

Regenerated `schema/` and `sdk/typescript/` with `BINGO_UPDATE_SCHEMA=1`
and the SDK, plugin-rpc, surface-rpc and mods dollar test generators.
No existing snapshots changed. No dependencies or discipline allowlists changed.

Command output summaries (all native commands below exited 0):

```text
$ cargo fmt --all -- --check
(no output)
$ cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo]
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo]
$ RUST_TEST_THREADS=2 cargo test --workspace --locked
5439 passed; 0 failed; 5 ignored (sum of 163 result lines)
TUI: 1168 passed; 0 failed; 2 ignored
$ cargo test -p bingo-surface-tui sealed --locked
9 passed; 0 failed
$ cargo test -p bingo-mods --lib guide::tests:: --locked
4 passed; 0 failed
$ bash scripts/check_discipline.sh
discipline ok (existing size/advisory warnings)
$ bash scripts/budget.sh
dependencies (unique, normal): 411 (max 411)
warm cargo check -p bingo-core: 0s (max 20s)
relink isolation: touching the TUI recompiled 0 crates for core
warn: target/debug exceeds the soft limit (7 GB; soft max 5)
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ bash scripts/tui-smoke.sh
tui-smoke ok (including all 10 M160 scenarios)
$ cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
exit 101: C build prerequisites unavailable on this macOS host
aws-lc-sys: fatal error: 'stdlib.h' / 'windows.h' file not found
rquickjs-sys: clang rejects MSVC /DWIN32_LEAN_AND_MEAN and /std:c11 flags
```

The first default-concurrency workspace run had two existing CLI failures:
`mentions::a_question_left_unanswered_is_chased_when_the_next_process_opens_the_room`
and `rooms::a_post_that_calls_on_the_holder_wakes_it_and_is_owed_an_answer`.
Both passed with two test threads. The full run passed after shortening the
mods guide to its existing limit; the final generic TUI fixture and guide edits
also passed the focused commands above. Core rejection notices are checked after
answering in tmux because the open host card can cover the transcript notice.

Windows remains a recorded deferral, not a passing build: the known aws-lc-sys
header blocker and rquickjs-sys C compiler flags stop the cross-check before it
can verify the workspace. The release Windows job remains the backstop.
The smoke used only its own `tmux -L` socket, `/dev/null` config and temporary home.
Build artifacts are removed with `cargo clean` after committing this record.
