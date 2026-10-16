# M183 — The frame

ADR-0095 §1, §3, §5. The prototype's frame on every screen: header, rule and
prompt line, always-on footer, chrome colours and glyphs.

## Goal

At 108×30 with the fake provider, the idle, running and finished-turn screens
match the prototype captures (`scratchpad/proto-audit/captures/proto/`) in
layout, glyphs and words, except the `KeyBar` row (M184) and the transcript fold
(M185).

## Bricks

1. **Facts.** The session's git branch (context plugin, read once per cwd
   change, none outside a repository); a model choice's scope (`session` or
   `default`) and the effort in the session summary. Fixture tests first.
2. **Theme tokens** (ADR-0095 §3): chrome tokens with light pairs and 256-colour
   nearest; prose stays `Color::Reset`, secondary stays SGR dim. Glyph table:
   `›` user, ` ▍ ` answer (presence when latest, ash when old), `✓ ◆ ◌ ✗`,
   `▸`, `■`, `⏸`, `⇄ ✎ ⟲ ⇣`.
3. **Header row**: `bingo▍` (▍ presence), `~`-shortened cwd, `· branch`, and
   right-aligned `<title> · session <short id>`. The wordmark block at the head
   of the transcript goes.
4. **Rule and prompt line** replace the rounded box: a `─` rule, then ` ▍ ` and
   the draft; placeholder `ask anything` idle, empty while a turn runs. The
   border breathing goes; the presence colour on `▍` carries it.
5. **Footer** always shows `⏵⏵ default` / `⏵⏵ accept edits` / `⏸ plan` (or the
   session's own mode in words), `◆ solo` or the crew faces, `▰▱▱▱▱▱▱▱ 14k /
   200k` (bad colour above 90%), and `model ∴ effort · this session` with a
   1.4 s glow flash on change. The middle slot and the notice line go; repo
   waiting and other-sessions-need-you move to toasts until M184 gives them pairs.
6. **shift+tab** cycles three modes, plus the start mode if outside them
   (ADR-0095 §5), in the permissions plugin's `Mode` slot.
7. **Narrow widths**: header drops title then branch; footer drops crew, then
   the bar, keeping mode and model.

## Files

- `crates/bingo-surface-tui/src/{frame.rs,view.rs,composer.rs,status.rs,theme.rs,activity.rs}`.
- `crates/bingo-context` (branch), `crates/bingo-sdk` (summary fields), `crates/bingo-permissions` (cycle).

## Exit criteria

- [ ] Snapshot per screen (idle, running, done, narrow 80×24) beside the prototype capture it matches.
- [ ] tmux drive at 108×30 and 80×24; captures pasted in Verified.
- [ ] tui-smoke and the PTY tests updated for the new composer probe.
- [ ] Gates as in M182; Windows all-target check.

## Non-goals

`KeyBar` and toast placement (M184); transcript grammar (M185).

## Risks

- Every surface test that probes `╭│ >` breaks at once: change the probe first,
  in one commit, before the drawing changes.
- Light terminals: each token's light pair is checked in a light-background run.
