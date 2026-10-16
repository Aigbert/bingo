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
5. **Footer** always shows `⏵⏵ default` / `⏵⏵ accept edits` / `⏸ plan`, `⏵⏵ bypass` in
   the bad colour (or the session's own mode in words), `◆ solo` or the crew faces, `▰▱▱▱▱▱▱▱ 14k /
   200k` (bad colour above 90%), and `model ∴ effort · this session` with a
   1.4 s glow flash on change. The middle slot and the notice line go; repo
   waiting and other-sessions-need-you move to toasts until M184 gives them pairs.
6. **shift+tab** cycles `default → accept edits → plan → bypass`, plus
   `dontAsk` after bypass only when the session started in it (ADR-0095 §5),
   in the permissions plugin's `Mode` slot; `/permission` still names all five.
7. **Narrow widths**: header drops title then branch; footer drops crew, then
   the bar, keeping mode and model.

## Files

- `crates/bingo-surface-tui/src/{frame.rs,view.rs,composer.rs,status.rs,theme.rs,activity.rs}`.
- `crates/bingo-context` (branch), `crates/bingo-sdk` (summary fields), `crates/bingo-permissions` (cycle).

## Implementation decisions

- Branch is a context-owned journal extension carrying the public `Location` slot
  `{cwd, branch}`. This keeps the TUI independent of a producer namespace; it never
  reads Git. Detached HEAD has no branch. A slot for another cwd is ignored.
  The observer reads the opening summary too; Git never delays session-start
  hooks, which avoids a reopen/start-hook ordering stall during child recovery.
- Model, scope and effort form a flattened `SessionModel` in the summary: one
  choice record keeps the wire flat and the struct cohesive. Scope stays in the spec
  so resume preserves the choice. Older journals deserialize without the optional facts.
- Keep existing hints and notices during the M184 transition; reserve the row
  below the prompt for them, without implementing KeyBar props or toast clocks.
- Necessary owner seams add `bingo-core` spec/model commands, the mod host adapter,
  TUI welcome site and render capability selection, generated contracts, tests
  and snapshots to Files. Mechanical summary constructors in dependent crates also
  gain the optional fields. These are direct dependencies, not reorganisations.
- Short transcripts grow from the header. Layout/ask hit testing, image measures
  and block cache invalidation follow the new three-cell prose indent; folds stay intact.
- Spec mutation lives in the existing core spec module, and frame glyphs form
  `ChromeGlyphs`; these groupings satisfy the existing discipline limits.
- Chrome grounds blend 75% with the measured terminal background, relative to
  the prototype dark ground (light reference: white), preserving its exact tokens
  on the reference ground. Indexed chrome follows the same background answers.
- Model choice is documented in the existing TUI guide page (ADR-0054).

## Exit criteria

- [x] Snapshot per screen (idle, running, done, narrow 80×24) beside the prototype capture it matches.
- [x] tmux drive at 108×30 and 80×24; captures pasted in Verified.
- [x] tui-smoke and the PTY tests updated for the new composer probe.
- [x] Native gates as in M182.
- [ ] Windows all-target check: 13 pass; seven blocked by cross-C toolchains
  (aws-lc-sys, rquickjs-sys, tree-sitter); per-crate output below.

## Non-goals

`KeyBar` and toast placement (M184); transcript grammar (M185).

## Risks

- Every surface test that probes `╭│ >` breaks at once: change the probe first,
  in one commit, before the drawing changes.
- Light terminals: each token's light pair is checked in a light-background run.

## Verified — 2026-10-16

Run in the worktree, provider and credential variables unset, fakes only.

```text
cargo fmt --all -- --check                                    exit 0
cargo check --workspace --all-targets --locked                exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings exit 0
cargo test --workspace --locked   6,228 passed; 0 failed; 6 ignored; exit 0
bash scripts/check_discipline.sh  discipline ok; exit 0
bash scripts/budget.sh            dependencies 422 / 422; budget ok; exit 0
cargo deny check                  advisories, bans, licenses, sources ok; exit 0
bash scripts/tui-smoke.sh         tui-smoke ok; exit 0 (probe now `▍ answer above`)
```

Generated schemas and TypeScript were refreshed with `BINGO_UPDATE_SCHEMA=1`;
the workspace run then found no drift. No dependency or ember allowlist changed.

Windows, `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`:
13 crates pass. Seven stop in C build scripts with no Rust diagnostic:
`bingo`, `bingo-channel-adapter`, `bingo-mods`, `bingo-plugin-rpc`,
`bingo-surface-print`, `bingo-surface-tui` (aws-lc-sys, rquickjs-sys) and
`bingo-permissions` (tree-sitter: no `stdio.h` for the target). Left unticked.

Snapshots beside the prototype captures (`k-idle`, `k-ask-edit-yes`,
`k-ask-edit-yes-done`), each at 108×30 and 80×24:
`crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_{idle,running,done}_{108x30,80x24}.snap`.

Release drive: private `tmux -L m183-<unique> -f /dev/null`, temporary HOME,
fake provider, demo repo on `main`; dark 108×30 and 80×24, light 108×30.

```text
108x30: rule row 27, prompt row 28, footer row 30
80x24:  rule row 21, prompt row 22, footer row 24
shift+tab: default → accept edits → plan → bypass → default
ANSI dark:  bypass 38;2;224;101;90; rule 38;2;58;52;43
ANSI light: bypass 38;2;176;47;36; rule 38;2;207;197;181
prose: answer starts with SGR 39; placeholder SGR 2 + 39
──────────────────────────────────────────────────────────────────────
 ▍ ask anything
 ⏵⏵ bypass   ◆ solo   ▰▱▱▱▱▱▱▱ 13k / 191k                fake-1 ∴ medium
```

Fix round: `win` declares `models."fake/fake-1".contextWindow = 200000`,
`bare` declares nothing.

```text
before first round, win:   ▱▱▱▱▱▱▱▱ 0k / 200k   …   fake-1
before first round, bare:  ▱▱▱▱▱▱▱▱ ? / ?       …   fake-1
`/permission ` after shift+tab:
 ❯ accept edits  acceptEdits                                    now
   bypass  bypassPermissions
row bypass + enter:        ⏵⏵ bypass  (ran /permission bypassPermissions)
row accept edits + enter:  ⏵⏵ accept edits
after first round, win:    ▰▱▱▱▱▱▱▱ 14k / 191k
/think high:               fake-1 ∴ high
```

Found, left for M184: the migration and mode notices still sit in the old row
and the mode notice can lag the footer; the denominator drops from the
catalogued window to the input window (200k → 191k) on the first measure —
core should publish a `ContextUsage` at session open so the bar reads one fact.
With no effort configured the summary carries none and the footer shows none.
