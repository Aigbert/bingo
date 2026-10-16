# M187 — The palette and the skills

ADR-0095; ADR-0084 §1. The `/` palette in the prototype's groups and words,
with skills kept (the person: "技能也得存在"; the prototype's list is a sketch).

## Goal

`/`, `?`, `ctrl+r`, `/mo`, `/think`, `/rename`, `/permission`, `/resume`,
`/login` match prototype scenarios palette-1 … palette-8; every skill is still
reachable from `/`.

## Bricks

0. **The palette sits under the prompt line** (the person, 2026-10-16: "把/slash
   command放到对话框最下面反而是最好的 就和claude code一样"). It takes the
   KeyBar's and the footer's rows while open, and the transcript gives up the
   rest of the room; the prompt line moves up with it. The `@` menu stays.
1. **Command use counts** per person (a `recent` group of up to 3 on a bare `/`).
2. Groups `recent`, `commands`, `skills`, `sessions`, `keys`; short
   descriptions with right-aligned values; plain-word key rows.
3. **Skills group** after commands, source (`project`, `yours`, `built in`) in
   ash; top 4 while searching then `· N more skills`; on bare `/` one row
   `▸ 12 skills · 9 guides`; `/guide-*` under `▸ guides` shown as
   `guide › memory`; name column = widest name capped at 16, descriptions cut on
   a word; a command beats a skill at an equal score;
   `bingo-skills/command.rs:60` puts the description, not the arg hint.
4. Inline argument levels for `/think` (only the model's levels), `/permission`,
   `/rename`, `/resume`, `/login`; Enter on `/model X` switches to the top match.
5. No match: `nothing matches “zz”`.

## Files

- `crates/bingo-mod-palette`, `crates/bingo-skills`, `crates/bingo-mod-models/commands.rs`, core command wording.

## Exit criteria

- [x] Snapshot per palette scenario plus the skills group with 0, 3 and 20 skills.
- [x] tmux drive with captures; per-key latency stays as M182 left it.
- [x] Gates as in M182: native gates pass; Windows TUI cross-check remains blocked.

## Non-goals

A new matcher algorithm beyond ranking ties; skill authoring.

## Risks

- Use counts are personal data: stored under the person's data dir, never in
  a project file; a test asserts the path.

## Verified

2026-10-16, macOS arm64, rebased onto dev `561da15c`. Provider credentials and
base URLs unset for every command; drives used a fake provider, temporary HOME,
data dir and cwd, and a private `tmux -L m187 -f /dev/null`, killed at the end.

- Snapshots: `palette_{commands,keys,sessions,mo,think,rename,permission,resume,login}`
  at 108×30 and 80×24, `palette_skills_{0,3,20}_{bare,search}` at both sizes,
  `palette_nothing_matches_80x24`. Use counts: `uses.rs` asserts the file sits
  under the person's data dir and never holds an argument.
- Captures: `docs/verification/m187/{120x40,80x24}/` — `/`, `/`+↓, `?`, ctrl+r,
  `/mo`, `/th`, `/de`, `/guide`, `/think `, `/rename `, `/permission `, `/resume `,
  `/login `, `/compact `, `/zz`, `/model `; `120x40/skills-{folded,open}.txt`
  walk ↓ from a bare `/` to `▸ 4 skills · 29 guides` and open it with Enter.
- Placement: the rule and prompt sit right above the palette, which ends on the
  last row; the status line and KeyBar are not drawn while it is open. Within one
  open the palette keeps its tallest height (the existing no-flicker rule), so a
  short argument level reached from a full `/` leaves blank rows under the list
  and the prompt does not jump while typing.
- Held-key latency (settle after the last key, 3 rounds, same machine minutes
  apart; `/` 30×↓, `/model ` 12×↓): before (dev `fa53061a`) 1045/1074/1053 ms and
  498/502/509 ms; after 1041/1069/1065 ms and 489/496/506 ms.

```text
exit 0 — cargo fmt --all -- --check: no output
exit 0 — cargo clippy --workspace --all-targets --locked -- -D warnings: Finished
exit 0 — RUST_TEST_THREADS=4 cargo test --workspace --locked --no-fail-fast: 116 suites, 6227 passed, 0 failed, 0 ignored
exit 0 — scripts/check_discipline.sh: discipline ok
exit 0 — scripts/budget.sh: 422/422 dependencies; warm core 0s/20s; isolation 0; budget ok
exit 0 — cargo deny check: advisories ok, bans ok, licenses ok, sources ok
exit 0 — TUI_SMOKE_SOCKET_PREFIX=m187-smoke scripts/tui-smoke.sh: tui-smoke ok
exit 0 — BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk/-p bingo-plugin-rpc/-p bingo-surface-rpc/-p bingo-mods: no generated file changed
exit 0 — cargo check -p bingo-sdk -p bingo-mod-palette -p bingo-skills --all-targets --locked --target x86_64-pc-windows-msvc: Finished
```

Windows TUI compilation is unverified (aws-lc-sys needs MSVC headers, as in
M182–M184). The smoke's `?` step now waits for the list's own words, since the
open palette takes the KeyBar's row. No push or merge was made.
