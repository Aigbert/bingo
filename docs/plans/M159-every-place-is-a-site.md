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
   a component), `TurnProgress { elapsedMs, tokensOut, interruptible,
   stopping }`, `SessionStatus` (kernel facts the status line shows), `Toast`,
   `Welcome`. Contract first: a JSON fixture and round-trip test per props
   type before the TUI uses it.
3. **Surfaces table.** `Component::surfaces()` with a test that every
   component lists at least one surface; all sites in this plan list
   `terminal`, `desktop`, `text`. Mods `validate` warns when a hook matches a
   component on a surface that never asks for it.
4. **Truthful surface.** The pulled pane request (`bingo-mods` `ui.rs`
   `$.ui.open`) no longer fixes `Terminal`; the surface that pulls completes
   the request with its own name. Headless test mount keeps `Terminal`.
5. **TUI routes through sites.** Every `ItemBody` arm in
   `transcript.rs` block building, the activity row's elapsed/token/interrupt
   tail, the status line's kernel facts, toasts and the welcome go through
   `component::Site::draw` with the built-in drawing as `own`. Existing
   snapshots are the oracle: no snapshot changes.
6. **Mod types.** Generated schema and TypeScript (`schema/*.json`,
   `sdk/typescript`) include the new components; `guide.md` lists the sites
   and the sealed note for M160.

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

- [ ] Every `ItemBody` variant except `Unknown` reaches a component site
      (test enumerates the variants).
- [ ] `Spinner` props match Claude Code's meaning; `TurnProgress` carries the
      tail facts; fixture tests for every new props type.
- [ ] `Component::surfaces()` covers every component; validate warning tested.
- [ ] No TUI snapshot changes; a black-box test (`crates/bingo/tests/
      mods_components.rs` style) shows a mod replacing a new site and another
      wrapping one with `Engine`.
- [ ] A site no hook matches makes no render request (count test).
- [ ] Gates: fmt, check, clippy -D warnings, test, check_discipline, budget,
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
