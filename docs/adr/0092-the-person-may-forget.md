# ADR-0092 — The person may read and forget

Status: accepted · 2026-10-15 · Plan: M178 · Amends: ADR-0049 §1, §5; ADR-0014; ADR-0088 §1

## Context

What rides into every request is three things no screen shows together:
memories (`/memory`), the instruction files loaded (`AGENTS.md`,
`CLAUDE.md`; no screen at all) and experience entries (`/experience`).
None says what it costs, and dropping a wrong memory means asking the
model or finding the file. ADR-0049 made the model the one writer, which
stands; it said nothing of a person removing one, and §5 deleted the
store's write side. No kernel door; the ratchet does not arise.

## Decision

1. **The model stays the only author.** The panel adds and edits nothing.
2. **The person may read.** `⏎` opens a row's text in the pager. No
   `$EDITOR`: the TUI does not hand the terminal to a foreign process
   mid-session until a later plan designs that hand-over.
3. **The person may forget.** `/memory forget [scope] <name>` removes the
   file and the index lines linking it; `/experience forget <id>` is the
   store call `ExperienceForget` makes. Instant; the panel's `d` asks
   once and runs the line.
4. **A write side for forget alone**: a pure `unlink(index, file)` and a
   temp-then-rename swap of the index. No slug maker, no index parser.
5. **Two typed services**, `bingo_services::{memory, experience}`: list
   (instruction files with estimated tokens), read and forget; the commands
   call the same functions.

## Consequences

- A forgotten memory may stay in this session's prompt until it is next
  composed (ADR-0048); the reply says so.
- The two plugins switch at the next start (ADR-0088 §1); the mod now.
- Token sizes are estimates, labelled `about`.

Refs: ADR-0014, ADR-0044, ADR-0048, ADR-0049, ADR-0082, ADR-0086, ADR-0088.

M178 clarification of ADR-0088: native, wire-free static `Pages` are catalogue
content, not a running service. Registering a guide does not defer a mod switch.
