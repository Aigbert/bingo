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

- [ ] Snapshot per palette scenario plus the skills group with 0, 3 and 20 skills.
- [ ] tmux drive with captures; per-key latency stays as M182 left it.
- [ ] Gates as in M182.

## Non-goals

A new matcher algorithm beyond ranking ties; skill authoring.

## Risks

- Use counts are personal data: stored under the person's data dir, never in
  a project file; a test asserts the path.
