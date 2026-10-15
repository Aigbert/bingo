# ADR-0091 — Rewind names what it takes back

Status: accepted · 2026-10-15 · Plan: M177 · Amends: ADR-0045 §3, ADR-0084 §1 (`Browse`)

## Context

`esc esc` opens a card of turns and `⏎` rewinds at once: the person never
sees which files go back, cannot take back only the conversation or only
the files, and is not told that what a shell line wrote stays. Reading
the transcript by turn, searching what is folded and copying a block has
no screen; ADR-0084 named `Browse` and never gave it props. ADR-0045's
recorded debt, the turns of a transcript derived twice, would become
three. No kernel door: `HostApi::rewind` exists and stays off the wire.

## Decision

1. **One turn walk**, `bingo_sdk::transcript::turns`, on `asked`
   (ADR-0089 §1); the checkpoints plugin and the TUI card derive none.
2. **A checkpoints service**, `bingo_services::checkpoints::touched(
   session, from) -> { files: [{ path, fate, changed }], shell_calls }`, read from
   the paths-only projection of `restore::plan`, never applied by it.
3. **`/rewind <turn> [both | conversation | files]`**, default `both`
   with ADR-0045 §3's order. `conversation` rewinds the journal and keeps
   the files, `files` restores and keeps the journal; each reply says so.
4. **A screen asks first.** The card's and Browse's rewind open a sheet:
   turns undone, each file's fate, `files a shell command wrote are not
   tracked` when one ran, the three modes. A typed line does not ask.
5. **The undone prompt returns to the composer** (M170's `Composer`).
6. **`Browse { session, selected, query, hit, count }`** joins the props,
   owner `bingo-mod-browse`, opened by `ctrl+f` and listed in `?`.

The wire exposes only safe `touched`; writes remain commands. Any future writing
service method must declare `unsafe_call: true`.

Previews pin the turn and transcript length, refresh on change and refuse while
busy. Drawing does no file I/O; search is cached and capped at 1000 matches.

## Consequences

- `conversation` leaves the tree and the transcript disagreeing on
  purpose; the reply names the files kept.
- One more `⏎` on the card; the typed command is unchanged for scripts.
- With the mod off, `ctrl+f` is the line search and the card submits.

Refs: ADR-0045, ADR-0082, ADR-0084, ADR-0086, ADR-0089.
