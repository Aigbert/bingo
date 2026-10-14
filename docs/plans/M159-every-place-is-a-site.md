# M159 — Every place is a site

ADR-0082 §§1–5.

## Goal

Every place the terminal draws in the transcript, the turn row and the status
line goes through one site, with fact-only props, so a mod can replace or wrap
the built-in drawing on any surface. Pixels do not change.

## Bricks

1. **Parity fix.** `SpinnerProps` keeps Claude Code's meaning: `message` is
   `Some(text)` only while a state overrides the word (stopping), else `None`;
   `suffix` is the ellipsis. Fixture test pins the wire shape.
2. **Fact props.** New sdk props, one per site, each derived from the event
   stream or `SessionState` with no key names, no preformatted text, no cells:
   `Reasoning`, `Compaction`, `Rewind`, `Interruption`, `QuestionAnswer`,
   `PermissionReceipt`, `Asset`, `ShellRun` (and any other `ItemBody` without
   a component: pending `Action`), `TurnProgress { elapsedMs, tokensOut, interruptible,
   stopping }`, `SessionStatus` (kernel facts the status line shows), `Toast`,
   `Welcome`. Contract first: a JSON fixture and round-trip test per props
   type before the TUI uses it.
3. **Surfaces table.** `Component::surfaces()` with a test that every
   component lists at least one surface; all sites in this plan list
   `terminal`, `desktop`, `text`. Mods `validate` warns when a hook matches a
   component on a surface that never asks for it.
4. **Truthful surface.** The pulled pane request (`bingo-mods` `ui.rs`
   `$.ui.open`) no longer fixes `Terminal`; the surface that pulls completes
   the request with its own name. A surface-free `PullRequest` becomes a
   `RenderRequest` only when pulled; this prevents a producer choosing the
   asking surface. Headless test mount keeps `Terminal`.
5. **TUI routes through sites.** Every `ItemBody` arm in
   `transcript.rs` block building, the activity row's elapsed/token/interrupt
   tail, the status line's kernel facts, toasts and the welcome go through
   `component::Site::draw` with the built-in drawing as `own`. Existing
   snapshots are the oracle: no snapshot changes.
6. **Mod types.** Generated schema and TypeScript (`schema/*.json`,
   `sdk/typescript`) include the new components; `guide.md` lists the sites
   and the inherited M160 sealed-site contract.

## Files

- `crates/bingo-sdk/src/render.rs`, `component/props.rs`, `component/*`.
- `crates/bingo-surface-tui/src/{transcript.rs,transcript/*,activity.rs,
  status.rs,toast.rs,welcome.rs,component.rs}`; small new modules where an
  arm grows a body.
- `crates/bingo-mods/src/{ui.rs,validate*}`, `guide.md`.
- `schema/`, `sdk/typescript/` regenerated; `docs/design/tui.md` one dated line.
- A fixture mod under `crates/bingo/tests/fixtures/mods/` that rewrites one
  new site (e.g. `Compaction`) and one wraps with `Engine`.

## Exit criteria

- [x] Every `ItemBody` variant except `Unknown` reaches a component site
      (test enumerates the variants).
- [x] `Spinner` props match Claude Code's meaning; `TurnProgress` carries the
      tail facts; fixture tests for every new props type.
- [x] `Component::surfaces()` covers every component; validate warning tested.
- [x] No TUI snapshot changes; a black-box test (`crates/bingo/tests/
      mods_components.rs` style) shows a mod replacing a new site and another
      wrapping one with `Engine`.
- [x] A site no hook matches makes no render request (count test).
- [x] Gates: fmt, check, clippy -D warnings, test, check_discipline, budget,
      cargo deny, `scripts/tui-smoke.sh`, Windows check (or recorded deferral).

## Non-goals

Interaction sites (composer, completion, switcher, pickers, search, pager,
form shell, layout); permission and sign-in (M160); a chat surface table;
viewport units; plugin-placed views (tasks, crew); visual changes;
new dependencies.

## Risks

- Props that leak drawing (a preformatted label, a key name) defeat the
  surface-neutral goal; review each props type against ADR-0082 §3.
- Routing hot paths through `Site::draw` must not add per-frame work when no
  hook matches; the memo key must not include fields that change every frame
  (`elapsedMs` ticks: key `TurnProgress` by turn and second, not by millisecond).
- `Spinner` parity changes what an existing mod reads; note it in the guide.
- Kernel feature nouns stay out of the sdk: no `task`, `room`, `team`.

## Verified — 2026-10-14

The pending `Action` was the remaining uncovered item shape; completed actions
keep `CommandOutput`. `PullRequest` has no surface and is completed by the asker,
so the schema generator's pre-release `Shown` amendment follows ADR-0082 §5;
`RenderRequest` remains an explicit plugin-schema root for existing readers.
`SessionStatus` includes title/cwd so replacing it also replaces child identity.
No snapshots, dependencies, discipline scripts or ember allowlists changed.

Command output summaries (all exit 0 except the recorded Windows deferral):

```text
$ cargo fmt --all -- --check
(no output)
$ cargo check --workspace --all-targets --locked
Finished `dev` profile in 18.30s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile in 18.96s
$ cargo test --workspace --locked
162 suite results: 5447 passed; 0 failed; 5 ignored
TUI: 1177 passed; 0 failed; 2 ignored; finished in 9.07s
$ bash scripts/check_discipline.sh
discipline ok (advisory length warnings)
$ bash scripts/budget.sh
dependencies: 411 (max 411); warm core check: 0s; relink isolation: 0 crates
 target/debug: 9 GB (soft max 5); test binaries: 147; budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ bash scripts/tui-smoke.sh
tui-smoke ok
$ cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
exit 101: aws-lc-sys lacks stdlib.h/windows.h; rquickjs-sys passes MSVC
/DWIN32_LEAN_AND_MEAN, /std:c11, /experimental:c11atomics to local clang.
$ cargo check -p bingo-sdk -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile in 14.06s
```

The smoke used the script's own `tmux -L` socket and `-f /dev/null`, temporary
HOME/XDG directories, and isolated shell startup files. An initial shell-only
attempt stopped at a mise trust prompt; no trust setting was changed.
The unchanged CLI test
`mentions::a_question_left_unanswered_is_chased_when_the_next_process_opens_the_room`
intermittently failed its resume-session assertion; the final complete workspace
rerun passed. Diagnostic-only edits were reverted. No live/real-credential tests ran.
Schemas and TypeScript were regenerated with `BINGO_UPDATE_SCHEMA=1` using the
sdk catalogue, mods dollar, plugin-rpc schema/wire and surface-rpc schema tests.
The Windows workspace check remains deferred to a Windows C toolchain; the
changed sdk/core code passes its cross-target check.

### Rebased verification — 2026-10-14

Rebased onto `dev` at `1a30f4f8` (includes `e6fc485d`). One exhaustive `Component::surfaces()` covers all 30 sites; Permission and SignIn list terminal, desktop and text.
Both sets of tests, `sealed()` and props-aware `answer_ok` remain. Schema/TypeScript generators reran with `BINGO_UPDATE_SCHEMA=1`; no snapshots changed. Smoke used the same isolated setup above.
The first discipline run rejected a tool name in the new sealed test state; replacing it with a generic sample tool fixed it. All gates below reran on that correction.

```text
$ cargo fmt --all -- --check: (no output)
$ cargo check --workspace --all-targets --locked: Finished `dev` profile in 8.02s
$ cargo clippy --workspace --all-targets --locked -- -D warnings: Finished `dev` profile in 6.41s
$ cargo test --workspace --locked: 163 suites; 5463 passed; 0 failed; 5 ignored. TUI: 1188 passed; 0 failed; 2 ignored; 8.75s
$ bash scripts/check_discipline.sh: discipline ok (advisory warnings)
$ bash scripts/budget.sh: 411 dependencies; warm core 1s; relink 0 crates; target/debug 10 GB (soft max 5); 148 test binaries; budget ok
$ cargo deny check: advisories ok, bans ok, licenses ok, sources ok
$ bash scripts/tui-smoke.sh: tui-smoke ok (includes M160 sealed cases)
$ cargo check --workspace --all-targets --target x86_64-pc-windows-msvc: exit 101, same aws-lc-sys headers / rquickjs-sys MSVC flags deferral
$ cargo check -p bingo-sdk -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked: Finished `dev` profile in 13.23s
```
