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

- [x] Test: every event and `$` method in the schemas has a description, and
      the laid types contain it as JSDoc.
- [x] Test: `bingo plugins types` lays `reference.md` and the four examples;
      the laid examples validate and their tests pass from the laid copy.
- [x] Blind check: a fresh Opus agent, given only `guide-mods` and an
      installed debug binary, writes the three mods in the Goal in a temp
      home; `bingo plugins validate` and `test` pass. Transcript summary in
      Verified, including every place it still had to guess.
- [x] Gates as in M182.

## Non-goals

An npm package; an LSP; changing the mod API itself; external-process plugins.

## Risks

- Size: the laid types grow several-fold. They live in the mod's `.bingo/`,
  which is already gitignored by its own `.gitignore`.
- A description written for Rust readers can read wrong in TS; the blind
  check is where that shows.

## Verified

**Bricks as built.** Each fact keeps one home and the rest is generated:
- Rust doc comments → schema `description` → JSDoc in `schema/*.json`, `wire.ts`,
  `events.ts`, `dollar-types.ts` (`bingo-schema-ts`), and `reference.md`. The
  `catalogue!` and `rows!` macros refuse a row without a doc. Face events of other
  services (`agents-control.*`, `team.section`, …) still carry none: their rows
  are not in the sdk catalogue, so out of scope.
- `dollar/nouns.rs` holds each noun's doc, `@example` and the hand-typed call
  signatures (`CALLS`), generated as `DollarNouns` and `DollarCalls`; `dollar.ts`
  and `ui.ts` are typed against them. `OpenPane`'s TS comments are pinned to the
  Rust `Open` fields by a test in `ui.rs`. `Component::summary` and element
  props come from Rust (`types/elements.rs`).
- Examples: `types/examples.rs` embeds the four `examples/plugins/*` with
  `include_str!` and lays them under `.bingo/examples/<name>/`; a test fails
  when a file there is neither laid nor kept home for a named reason.
- `reference.md` (~2,500 lines): events, `$` nouns and methods, elements, sites.
- The guide: 173 lines, opens with the four-step path and the asks table.
- Kept: `schema/plugin-2.json` is the frozen protocol; doc comments respell a
  documented unit variant as `const` and a free value as `{description}`, so
  `bingo-plugin-rpc`'s freeze test compares a normalised spelling (`spelled`);
  `BINGO_UPDATE_SCHEMA=1` rewrites it, so it was restored after each regen.
- `index.d.ts` exports `Wire`, so a mod can name a site's props type.

**Tests.** `types::tests::the_laid_types_carry_every_event_and_method_doc_as_jsdoc`
(every catalogue event, `$` row, call and noun doc and example is a JSDoc line
in the laid `.bingo/types/bingo/`); the `catalogue!` and `rows!` macros make a
missing doc a compile error; `dollar::tests::every_noun_has_a_doc_and_an_example_
and_every_call_one_doc`; `reference::tests::*`; `elements::tests::*`;
`examples::tests::*`; `plugin_kit::types_lay_the_reference_and_examples_that_
validate_and_pass`; `plugin_kit::every_noun_example_runs_in_the_test_runtime`
(each `@example` runs in `bingo plugins test`; `model`, `http`, `env`,
`process`, `agent`, `mcp` need grants or services a test host lacks and are
skipped by `runs: false`). The `$.command` example once ran its own command and
failed (`callback command/hello is unavailable in this generation`): a command
registered in a test body cannot run there, so the example only registers.

**Blind check** (fresh Opus agent; only the guide, the debug binary, a temp HOME):
pane PASS, deny-rm PASS, slash PASS (`validate` and `test` exit 0 each; the
pane test was proven able to fail by breaking a count). Re-run on the final
binary: six exits 0. Its guesses, all right, and what was done:
1. What a command handler receives (the args string): probed. Fixed: noun doc,
   guide table row, `CommandRegistration` JSDoc (`description` is the `hint`).
2. `$.command.run` args `{}` vs a string: fixed in the guide and `pane` example.
3. How to assert a toast: dumped the snapshot. Fixed: guide names `signals`.
4. `tool.call` deny vs `tool.check` deny: guide row now says which asks first.
5. Whether a stateless mod needs `PluginState`: guide now says `{}`.
6. `place`/`size` values: found by grepping `wire.ts`. Fixed: docs on `Place`,
   `PaneSize`, `Span`, `Length`, `Rect` in `slot/size.rs`, and a guide line.
Not fixed, out of scope: a test failure's line number is the compiled line, not
the source line; `test requires an unanswered interaction` does not name the
call; `validate` on a mod without `types/index.d.ts` points at `1:1` of that
missing file; `plugins types` on a directory without `plugin.json` fails with a
bare `No such file or directory`. Root `tsconfig.json` includes `**/*.ts`;
TypeScript wildcards skip `.bingo/`, so the laid examples do not clash (local
`tsc --noEmit` clean on all three blind mods).

**Gates** (worktree, final tree):
```
cargo fmt --all -- --check                                   exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings  exit 0
cargo test --workspace --locked                              exit 0 (115 result lines ok)
scripts/check_discipline.sh                                  exit 0  discipline ok
scripts/budget.sh                                            exit 0  budget ok (target/debug soft-max warn)
cargo deny check                                             exit 0  advisories ok, bans ok, licenses ok, sources ok
```
