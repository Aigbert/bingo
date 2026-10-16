# M190 — The mod author's kit

ADR-0054 (guide pages as skills). The person, 2026-10-16, comparing Claude
Code's mod skill: "我们的mod说明感觉很少，agent在写mod的时候还得自己去翻源码".

## Goal

An agent that has read only `guide-mods` and the files `bingo plugins types
<dir>` lays writes a working mod (a pane, a tool-call rule, a slash command)
without opening bingo's source. Every fact it needs is on the local disk,
greppable, next to the mod.

## Today

- The laid types (`.bingo/types/bingo/*.ts`) are shapes only: `dollar.ts` and
  `dollar-types.ts` carry two comment lines between them, while
  `schema/dollar.json` and `schema/events.json` hold 88 and 278 descriptions
  that never reach the author.
- The page is capped under 200 lines (`crates/bingo/src/page_tests.rs`), so
  there is no long form.
- The page sends authors to `examples/plugins/` "in the repository"; an
  installed binary has no repository.

## Bricks

1. **Descriptions reach the types.** Each event, its input and result fields,
   each `$` method and its parameters and result, and each element's props
   carry their schema description as a JSDoc comment in the laid types. The
   text has one home: the Rust doc comment the schema is generated from. A
   missing description is a test failure, not a blank comment.
2. **One example per `$` noun**, as an `@example` on the noun, kept in the
   same home and type-checked with the laid examples (brick 4).
3. **A reference, laid beside the types.** `.bingo/reference.md`, generated
   from the same schemas and the element table: every event (matcher fields,
   input, result, what `next` does), every `$` method, every site and
   component with its props. Generated, so it cannot drift from the types.
4. **Examples travel with the binary.** `examples/plugins/{tool-call,band,
   pane,ts-status}` are embedded and laid under `.bingo/examples/<name>/`.
   Their existing tests stay the proof that they load, validate and pass.
5. **The page leads with the path.** `guide-mods` opens with: lay the kit
   (`bingo plugins types <dir>`), grep `.bingo/types/` for the name at hand,
   read `.bingo/reference.md`, start from `.bingo/examples/`; then a table
   "the person asks for → what it is → which example", as Claude Code's
   skill does. The page stays under 200 lines; detail moves to the reference.

## Files

- `crates/bingo-mods/src/types/{render.rs,files.rs}`, `crates/bingo-mods/src/types.rs`
- `sdk/typescript/src/{dollar-types.ts,events.ts,ui.ts}` only where a comment
  cannot be generated; the schema generator wherever `schema/*.json` is built.
- `crates/bingo/src/plugins/types.rs`, `crates/bingo-mods/src/guide.md`.
- No new dependency: examples are embedded with `include_str!`.

## Exit criteria

- [ ] Test: every event and `$` method in the schemas has a description, and
      the laid types contain it as JSDoc.
- [ ] Test: `bingo plugins types` lays `reference.md` and the four examples;
      the laid examples validate and their tests pass from the laid copy.
- [ ] Blind check: a fresh Opus agent, given only `guide-mods` and an
      installed debug binary, writes the three mods in the Goal in a temp
      home; `bingo plugins validate` and `test` pass. Transcript summary in
      Verified, including every place it still had to guess.
- [ ] Gates as in M182.

## Non-goals

An npm package; an LSP; changing the mod API itself; external-process plugins.

## Risks

- Size: the laid types grow several-fold. They live in the mod's `.bingo/`,
  which is already gitignored by its own `.gitignore`.
- A description written for Rust readers can read wrong in TS; the blind
  check is where that shows.
