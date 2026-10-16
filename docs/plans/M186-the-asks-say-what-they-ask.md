# M186 — The asks say what they ask

ADR-0095 §4; ADR-0084 §4, §5. Every ask in the prototype's words.

## Goal

The edit, bash, dangerous-command, question, multi-select, form and crew asks
match prototype scenarios ask-1 … ask-8 in title, question, options, keys.

## Bricks

1. **Scope phrase** on permission requests (`allow edits under src/ this
   session`, `allow cargo test … this session`) and a destructive command's
   size and count, written by the permissions plugin; fixture tests.
2. Titles `◆ edit src/main.rs … needs you`; questions `Make this edit to
   src/main.rs?`, `Run this command?`; relative paths; hunk-only diffs; inline
   `ctrl+o` expand.
3. Receipts: `you: yes to src/ this session` and the toast `edits under src/
   won't ask again this session`; `✗ edit … you: no, keep it`.
4. Questions: inline free-text option; multi-select `[x]`/`[ ]` with `space
   tick` and `⏎ answer with N`; a structured answer with the form's asker.
5. Set-aside ask: activity `▍ edit src/main.rs is waiting for you`, Esc goes
   back to it (ADR-0095 §4), `⏎ queue this message`.
6. Crew asks show every name with its face.

## Files

- `crates/bingo-permissions`, `crates/bingo-mod-ask`, `crates/bingo-sdk` (answer shape).
- `crates/bingo-surface-tui/src/{dialog.rs,preview.rs,ask/*,sealed.rs}`.

## Exit criteria

- [ ] Snapshot per ask scenario; a test per Esc rule in ADR-0095 §4.
- [ ] M180's "keys never enter an ask" tests unchanged and green.
- [ ] tmux drive of ask-1…5 and ask-7 with captures; gates as in M182.

## Non-goals

New permission rules or modes.

## Risks

- The scope phrase must name exactly what the rule will allow; it is generated
  from the rule, never written beside it.
