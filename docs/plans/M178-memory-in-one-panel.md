# M178 — Memory in one panel

ADR-0092, ADR-0049, ADR-0014, ADR-0082, ADR-0086. Needs M167; M170 for
the context estimate (`ContextPart`); M177 for the `Pager` surface action.

## Goal

What the agent carries into every request, seen in one place and
removable by the person. `/memory` opens a panel with three groups:
memories (scope, name, kind, description, path), the instruction files
loaded (`AGENTS.md`, `CLAUDE.md`, each with its estimated tokens), and
this project's experience entries. `⏎` reads any row in the pager, `d`
forgets a memory or an entry after a confirmation. `/memory forget
<name>` and `/experience forget <id>` do the same by line. The model
stays the only author (ADR-0092 §1). The feature is the built-in mod
`bingo-mod-memory`, a plugin view; with it off `/memory` and
`/experience` draw their tables as today.

```
 memory · about 6.1k tokens carried                       ⏎ read  d forget
 memories (4)
❯ project  feedback  no-sonnet-workers   Workers are Opus only      320
  user     user      role                Rust reviewer, prefers…    110
 instructions (2)
  ~/.bingo/AGENTS.md                                             about 900
  ./AGENTS.md                                                   about 4.2k
 experience (3)
  01J9…  active  fixture before impl for wire formats  3 ok      about 80
 ──────────────────────────────────────────────────────────────────────
 forget "no-sonnet-workers"? the file and its index line go.  ⏎ yes  esc
```

## Bricks

1. **Drop an index line, pure.** `crates/bingo-context/src/memory/
   index.rs::unlink(index: &str, file: &str) -> String`: every line whose
   markdown link targets `file` goes, nothing else changes (blank lines,
   headings, other links kept byte for byte). Table test incl. two links
   on one line, a link with a title, CRLF.
2. **Forget a memory.** `crates/bingo-context/src/memory/store.rs::
   forget(scope, name)`: remove the file, then write the index through a
   same-directory temp file and `rename` (ADR-0092 §4). A missing file is
   `NOT_FOUND`; a name in both scopes without a scope is
   `INVALID_INPUT` naming both. Tests in a temp dir, unix and Windows
   rename semantics (replace an existing file).
3. **Instruction files measured.** `crates/bingo-context/src/
   instructions.rs`: `loaded(cwd) -> Vec<Loaded { path, tokens }>` from
   the same walk the contributor makes, tokens by the estimator the
   context parts use (M170 brick 1). Test: two files, order kept.
4. **Typed services, contracts first.** `crates/bingo-services/src/
   {memory.rs,experience.rs}`: memory `list(cwd) -> { memories:
   [MemoryRow { scope, name, kind, description, path, tokens }],
   instructions: [Loaded] }`, `forget(scope?, name)`; experience
   `list(cwd) -> [EntryRow { id, status, summary, outcomes, tokens }]`, `forget(id)`.
   `find(host)` native-or-wire; fixtures under `fixtures/{memory,
   experience}/` before any implementation.
5. **Commands on the same functions.** `/memory forget [user|project]
   <name>` in `crates/bingo-context/src/memory/command.rs`; `/experience
   forget <id>` in `crates/bingo-experience/src/command.rs` over
   `store.rs::delete` (the call `tools/forget.rs` makes). Both instant,
   both answer what went and, for a memory, that this session's prompt may
   keep it until next composed. Bare commands unchanged. Parity test:
   command and service answer the same text.
6. **Rows, pure.** `crates/bingo-mod-memory/src/rows.rs::rows(listing,
   entries) -> Vec<Group>` with the header total (`about N tokens
   carried`). Table test: empty scopes, malformed memory file (shown with
   its parse error, not hidden), no experience plugin.
7. **Keys.** `keys.rs` through `onKey`: `↑↓`, `⏎` → `Pager { title, text
   }` with the file read by the service, `d` on a memory or entry → the
   confirmation line, `⏎` there runs `/memory forget <scope> <name>` or
   `/experience forget <id>`; `d` on an instruction file → toast `edit
   the file itself`. Table test (row, key) → outcome.
8. **Drawing.** `draw.rs`: a `Pane` view, groups with counts, tokens
   right-aligned, a confirmation as the last line. Snapshots at 108×30
   and 80×24: full, empty, confirming, malformed.
9. **Guide.** `guide.md` (<200 lines) by `guide.rs`, as `bingo-mod-palette`.

## Files

- `crates/bingo-context/src/{instructions.rs,lib.rs}`, `memory/{index.rs
  (new),store.rs,command.rs,mod.rs}`.
- `crates/bingo-experience/src/{command.rs,lib.rs}`.
- `crates/bingo-services/src/{lib.rs,memory.rs,experience.rs}`, fixtures.
- TUI: `run/surface.rs` (`Pager { title, text }` beside M177's `{ item }`).
- New `crates/bingo-mod-memory/src/{lib.rs,rows.rs,keys.rs,draw.rs,
  guide.rs,guide.md}`; listed in `crates/bingo/src/main.rs`; +1 member in
  `scripts/budget.toml`, no dependency.
- `docs/adr/0049-memory-has-one-writer.md` gains the amendment line.

## Exit criteria

- [ ] `unlink`, `rows`, keys tables; `forget` temp-dir tests; parity
      tests for both commands; service fixtures.
- [ ] `TestBackend`: `/memory` opens the panel; `⏎` pages a memory; `d`
      then `⏎` removes file and index line, the row goes; `d` on an
      instruction file is refused with the toast.
- [ ] `/memory forget` of a name in both scopes answers both scopes.
- [ ] `enabledPlugins: {"bingo.mod.memory": false}`: both tables draw.
- [ ] tmux drive on an isolated socket, temp HOME seeded with two
      memories, an `AGENTS.md` and one experience entry, fake provider;
      forget one of each; captures pasted.
- [ ] `bash scripts/tui-smoke.sh`; `cargo fmt --all -- --check`; `cargo
      check --workspace --all-targets --locked`; `cargo clippy --workspace
      --all-targets --locked -- -D warnings`; `cargo test --workspace
      --locked`; `scripts/check_discipline.sh`; `scripts/budget.sh`;
      `cargo deny check`.
- [ ] `cargo check -p bingo-context -p bingo-experience -p bingo-mod-memory
      --all-targets --target x86_64-pc-windows-msvc --locked` (rename).

## In-flight dependencies

- M166: none.
- M170: the token estimate and `ContextPart`; land after it.
- M173: `bingo-context` and `bingo-experience` gain services, so their
  switches say `off at next start`; the mod switches now.
- M174: the confirmation's `⏎` resolves against the current drawing, so a
  row that vanished under it forgets nothing.

## Non-goals

Writing or editing a memory or an entry from the panel; `$EDITOR`;
editing instruction files; memories of other projects.

## Risks

- A forget races the model writing the same index in a running turn;
  the swap is whole-file, so the later writer wins and the panel
  re-lists after every forget.
- Token sizes disagree with the server's count; every number says
  `about`.
- A person forgets a memory the model then rewrites from the same
  conversation; ADR-0049 §2's teaching says a correction is kept, and
  the reply suggests saying so in the chat.
