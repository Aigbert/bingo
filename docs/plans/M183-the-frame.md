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

The commands below ran sequentially in this worktree. Tests used local fakes
and fixtures; provider/credential environment variables were removed for the
workspace test gate. The first run exposed an inherited-provider-variable
fixture assumption; the isolated rerun passed. Frame changes also exposed
PTY blank-cell handling and temporary cwd names in completion snapshots;
both probes were corrected and rerun without accepting variable paths.

### Command results

```text
cargo fmt --all -- --check
  exit 0
cargo check --workspace --all-targets --locked
  Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings
  Finished dev profile; exit 0
cargo test --workspace --locked
  197 result groups: 6,228 passed; 0 failed; 6 ignored; exit 0
bash scripts/check_discipline.sh
  catalogue entry declarations ok; discipline ok; exit 0
bash scripts/budget.sh
  dependencies: 422 / 422; warm core check: 0s / 20s
  TUI touch rebuilt 0 core dependencies
  target/debug: 13 GB (soft max 5; warning); 186 test binaries
  budget ok; exit 0
cargo deny check
  advisories ok, bans ok, licenses ok, sources ok; exit 0
bash scripts/tui-smoke.sh
  tui-smoke ok; exit 0
```

Schema and TypeScript generators ran with `BINGO_UPDATE_SCHEMA=1` through the
existing SDK, store, plugin-RPC, mods and surface-RPC generator tests (500 passed,
one ignored in the final generator run). The subsequent workspace suite checked
that the committed outputs have no drift. No dependency or ember allowlist changed.
The smoke harness used a disposable HOME and `SHELL=/bin/sh`; this avoids a local
shell-initialisation trust prompt. Its pending-permission border probe also now
checks `▍ answer above` instead of the removed `│` composer border.

Windows command, run once for **each** crate below:
`cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`.

| Crate | Result |
|---|---|
| `bingo` | deferred (exit 101): `aws-lc-sys v0.44.0`, `rquickjs-sys v0.14.0` |
| `bingo-agents` | pass (exit 0) |
| `bingo-channel-adapter` | deferred (exit 101): `aws-lc-sys v0.44.0` |
| `bingo-checkpoints` | pass (exit 0) |
| `bingo-context` | pass (exit 0) |
| `bingo-core` | pass (exit 0) |
| `bingo-demo-ui` | pass (exit 0) |
| `bingo-experience` | pass (exit 0) |
| `bingo-mods` | deferred (exit 101): `rquickjs-sys v0.14.0`, `aws-lc-sys v0.44.0` |
| `bingo-permissions` | deferred (exit 101): `tree-sitter v0.26.13`, `tree-sitter-bash v0.25.1` |
| `bingo-persona` | pass (exit 0) |
| `bingo-plugin-rpc` | deferred (exit 101): `aws-lc-sys v0.44.0` |
| `bingo-rooms` | pass (exit 0) |
| `bingo-sdk` | pass (exit 0) |
| `bingo-skills` | pass (exit 0) |
| `bingo-store-jsonl` | pass (exit 0) |
| `bingo-surface-print` | deferred (exit 101): `aws-lc-sys v0.44.0` |
| `bingo-surface-rpc` | pass (exit 0) |
| `bingo-surface-tui` | deferred (exit 101): `aws-lc-sys v0.44.0` |
| `bingo-tasks` | pass (exit 0) |

Thirteen Windows all-target checks passed. The remaining checks stop in C build
scripts: the known aws-lc/MSVC and QuickJS cross-toolchain blockers, plus
permissions' tree-sitter dependencies (`stdio.h` / `stdlib.h` unavailable for the
Windows target). A separate permissions `--lib` check reaches the same missing
headers. No Windows Rust diagnostic was reported, but those seven all-target
checks remain unverified; the Windows exit criterion is intentionally unticked.

The final full workspace run passed the child-recovery regression after branch
publication moved out of session-start hooks. The focused recovery check also
passed (one test, 1.15 s); the stalled earlier run is not counted as a pass.

### Prototype and frame snapshots

References are the supplied `proto-audit/captures/proto/` captures. The audit's
C1–C15 informed this frame; KeyBar pairs/toast placement and transcript folding
remain M184/M185. The running placeholder follows brick 4 (empty), even though
that earlier prototype capture still contains `ask anything`. Browser-drawn
carets correspond to the terminal's native caret and are absent from pane text.

| State | Prototype reference | TestBackend frame |
|---|---|---|
| idle, 108x30 | `k-idle.txt` (108×30 reference) | [108x30 snapshot](../../crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_idle_108x30.snap) |
| idle, 80x24 | `k-idle.txt` (108×30 reference) | [80x24 snapshot](../../crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_idle_80x24.snap) |
| running, 108x30 | `k-ask-edit-yes.txt` (108×30 reference) | [108x30 snapshot](../../crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_running_108x30.snap) |
| running, 80x24 | `k-ask-edit-yes.txt` (108×30 reference) | [80x24 snapshot](../../crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_running_80x24.snap) |
| done, 108x30 | `k-ask-edit-yes-done.txt` (108×30 reference) | [108x30 snapshot](../../crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_done_108x30.snap) |
| done, 80x24 | `k-ask-edit-yes-done.txt` (108×30 reference) | [80x24 snapshot](../../crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__frame__m183__frame_done_80x24.snap) |

### Isolated release drive

`cargo build --release --locked -p bingo`; disposable HOME/data/config and
`~/src/demo` repository on branch `main`; fake provider with a five-second delay.
A private `tmux -L m183-<unique> -f /dev/null` socket owned every pane. No live
provider or personal bingo directory was used. The harness exited its own panes,
killed only its own server and removed its temporary home afterward.

Below, each full 108×30 prototype capture is beside the unnormalised release
pane at the requested size. Session IDs, titles, model names and measured context
are live fixture values; the seeded TestBackend snapshots pin the prototype's
model/effort and 14k/200k values. The narrow reference remains 108 columns because
the approved prototype has no 80-column mode. Blank rows are retained. Pasted
capture evidence deliberately makes this record exceed the usual plan length.

#### idle, 108x30: `k-idle.txt` │ release

```text
01  bingo▍ ~/src/demo · main                                           label the total in main · session 3f2a   │  bingo▍ ~/src/demo · main                                                                     session 01M5
02                                                                                                              │
03  › what prints the total?                                                                                    │
04    ▸ 2 steps · read · grep                                                                              3s   │
05  ▍ main in src/main.rs prints it with println!.                                                              │
06                                                                                                              │
07  › label the total in main and make sure the tests still pass                                                │
08    ▸ 5 steps · read · grep · edit +1 −1 · test 3 passed · run ok                                        7s   │
09  ▍ Done. main now prints a labelled total.                                                                   │
10                                                                                                              │
11                                                                                                              │
12                                                                                                              │
13                                                                                                              │
14                                                                                                              │
15                                                                                                              │
16                                                                                                              │
17                                                                                                              │
18                                                                                                              │
19                                                                                                              │
20                                                                                                              │
21                                                                                                              │
22                                                                                                              │
23                                                                                                              │
24                                                                                                              │
25                                                                                                              │
26                                                                                                              │
27 ──────────────────────────────────────────────────────────────────────────────────────────────────────────── │ ────────────────────────────────────────────────────────────────────────────────────────────────────────────
28  ▍ ▌ask anything                                                                                             │  ▍ ask anything
29                                  / commands · ? keys · ctrl+r sessions · shift+tab mode · alt+m next model   │ ? for shortcuts
30  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 14k / 200k                                      claude-sonnet-5-5 ∴ medium   │  ⏵⏵ default   ◆ solo   ▱▱▱▱▱▱▱▱ ? / ?                                                      fake-1 ∴ medium
```

#### running, 108x30: `k-ask-edit-yes.txt` │ release

```text
01  bingo▍ ~/src/demo · main                                           label the total in main · session 3f2a   │  bingo▍ ~/src/demo · main        label the total in main and make sure the tests still pass · session 01M5
02                                                                                                              │
03  › what prints the total?                                                                                    │  › label the total in main and make sure the tests still pass
04    ▸ 2 steps · read · grep                                                                              3s   │
05  ▍ main in src/main.rs prints it with println!.                                                              │
06                                                                                                              │
07  › label the total in main and make sure the tests still pass                                                │
08    ✓ read src/main.rs                                                                                        │
09    ✓ grep total                                                                                              │
10    ✓ edit src/main.rs  +1 −1                                                                      you: yes   │
11                                                                                                              │
12                                                                                                              │
13                                                                                                              │
14                                                                                                              │
15                                                                                                              │
16                                                                                                              │
17                                                                                                              │
18                                                                                                              │
19                                                                                                              │
20                                                                                                              │
21                                                                                                              │
22                                                                                                              │
23                                                                                                              │
24                                                                                                              │
25                                                                                                              │
26  ▍ burrowing · 1s                                                                                 esc stop   │  ▍ sifting · 0s · esc to interrupt
27 ──────────────────────────────────────────────────────────────────────────────────────────────────────────── │ ────────────────────────────────────────────────────────────────────────────────────────────────────────────
28  ▍ ▌ask anything                                                                                             │  ▍
29                                                                               esc stop · ⏎ queue a message   │ ? for shortcuts
30  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 14k / 200k                                      claude-sonnet-5-5 ∴ medium   │  ⏵⏵ default   ◆ solo   ▱▱▱▱▱▱▱▱ ? / ?                                                      fake-1 ∴ medium
```

#### done, 108x30: `k-ask-edit-yes-done.txt` │ release

```text
01  bingo▍ ~/src/demo · main                                           label the total in main · session 3f2a   │  bingo▍ ~/src/demo · main        label the total in main and make sure the tests still pass · session 01M5
02                                                                                                              │
03  › what prints the total?                                                                                    │  › label the total in main and make sure the tests still pass
04    ▸ 2 steps · read · grep                                                                              3s   │
05  ▍ main in src/main.rs prints it with println!.                                                              │  ▍ Done. main now prints a labelled total.
06                                                                                                              │
07  › label the total in main and make sure the tests still pass                                                │ ▍ Worked for 5s · done 11:49
08    ▸ 4 steps · read · grep · edit · bash · you: yes                                                     1s   │
09  ▍ Done. main now prints a labelled total, and the three tests still pass.                                   │
10                                                                                                              │
11                                                                                                              │
12                                                                                                              │
13                                                                                                              │
14                                                                                                              │
15                                                                                                              │
16                                                                                                              │
17                                                                                                              │
18                                                                                                              │
19                                                                                                              │
20                                                                                                              │
21                                                                                                              │
22                                                                                                              │
23                                                                                                              │
24                                                                                                              │
25                                                                                                              │
26                                                                                                              │
27 ──────────────────────────────────────────────────────────────────────────────────────────────────────────── │ ────────────────────────────────────────────────────────────────────────────────────────────────────────────
28  ▍ ▌ask anything                                                                                             │  ▍ ask anything
29                                  / commands · ? keys · ctrl+r sessions · shift+tab mode · alt+m next model   │ ? for shortcuts
30  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 14k / 200k                                      claude-sonnet-5-5 ∴ medium   │  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 13k / 191k                                                 fake-1 ∴ medium
```

#### idle, 80x24: `k-idle.txt` │ release

```text
01  bingo▍ ~/src/demo · main                                           label the total in main · session 3f2a   │  bingo▍ ~/src/demo · main                                         session 01M5
02                                                                                                              │
03  › what prints the total?                                                                                    │
04    ▸ 2 steps · read · grep                                                                              3s   │
05  ▍ main in src/main.rs prints it with println!.                                                              │
06                                                                                                              │
07  › label the total in main and make sure the tests still pass                                                │
08    ▸ 5 steps · read · grep · edit +1 −1 · test 3 passed · run ok                                        7s   │
09  ▍ Done. main now prints a labelled total.                                                                   │
10                                                                                                              │
11                                                                                                              │
12                                                                                                              │
13                                                                                                              │
14                                                                                                              │
15                                                                                                              │
16                                                                                                              │
17                                                                                                              │
18                                                                                                              │
19                                                                                                              │
20                                                                                                              │
21                                                                                                              │ ────────────────────────────────────────────────────────────────────────────────
22                                                                                                              │  ▍ ask anything
23                                                                                                              │ ? for shortcuts
24                                                                                                              │  ⏵⏵ default   ◆ solo   ▱▱▱▱▱▱▱▱ ? / ?                          fake-1 ∴ medium
25                                                                                                              │
26                                                                                                              │
27 ──────────────────────────────────────────────────────────────────────────────────────────────────────────── │
28  ▍ ▌ask anything                                                                                             │
29                                  / commands · ? keys · ctrl+r sessions · shift+tab mode · alt+m next model   │
30  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 14k / 200k                                      claude-sonnet-5-5 ∴ medium   │
```

#### running, 80x24: `k-ask-edit-yes.txt` │ release

```text
01  bingo▍ ~/src/demo · main                                           label the total in main · session 3f2a   │  bingo▍ ~/src/demo · main                                         session 01M5
02                                                                                                              │
03  › what prints the total?                                                                                    │  › label the total in main and make sure the tests still pass
04    ▸ 2 steps · read · grep                                                                              3s   │
05  ▍ main in src/main.rs prints it with println!.                                                              │
06                                                                                                              │
07  › label the total in main and make sure the tests still pass                                                │
08    ✓ read src/main.rs                                                                                        │
09    ✓ grep total                                                                                              │
10    ✓ edit src/main.rs  +1 −1                                                                      you: yes   │
11                                                                                                              │
12                                                                                                              │
13                                                                                                              │
14                                                                                                              │
15                                                                                                              │
16                                                                                                              │
17                                                                                                              │
18                                                                                                              │
19                                                                                                              │
20                                                                                                              │  ▍ smouldering · 0s · esc to interrupt
21                                                                                                              │ ────────────────────────────────────────────────────────────────────────────────
22                                                                                                              │  ▍
23                                                                                                              │ ? for shortcuts
24                                                                                                              │  ⏵⏵ default   ◆ solo   ▱▱▱▱▱▱▱▱ ? / ?                          fake-1 ∴ medium
25                                                                                                              │
26  ▍ burrowing · 1s                                                                                 esc stop   │
27 ──────────────────────────────────────────────────────────────────────────────────────────────────────────── │
28  ▍ ▌ask anything                                                                                             │
29                                                                               esc stop · ⏎ queue a message   │
30  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 14k / 200k                                      claude-sonnet-5-5 ∴ medium   │
```

#### done, 80x24: `k-ask-edit-yes-done.txt` │ release

```text
01  bingo▍ ~/src/demo · main                                           label the total in main · session 3f2a   │  bingo▍ ~/src/demo · main                                         session 01M5
02                                                                                                              │
03  › what prints the total?                                                                                    │  › label the total in main and make sure the tests still pass
04    ▸ 2 steps · read · grep                                                                              3s   │
05  ▍ main in src/main.rs prints it with println!.                                                              │  ▍ Done. main now prints a labelled total.
06                                                                                                              │
07  › label the total in main and make sure the tests still pass                                                │ ▍ Worked for 5s · done 11:49
08    ▸ 4 steps · read · grep · edit · bash · you: yes                                                     1s   │
09  ▍ Done. main now prints a labelled total, and the three tests still pass.                                   │
10                                                                                                              │
11                                                                                                              │
12                                                                                                              │
13                                                                                                              │
14                                                                                                              │
15                                                                                                              │
16                                                                                                              │
17                                                                                                              │
18                                                                                                              │
19                                                                                                              │
20                                                                                                              │
21                                                                                                              │ ────────────────────────────────────────────────────────────────────────────────
22                                                                                                              │  ▍ ask anything
23                                                                                                              │ ? for shortcuts
24                                                                                                              │  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 13k / 191k                     fake-1 ∴ medium
25                                                                                                              │
26                                                                                                              │
27 ──────────────────────────────────────────────────────────────────────────────────────────────────────────── │
28  ▍ ▌ask anything                                                                                             │
29                                  / commands · ? keys · ctrl+r sessions · shift+tab mode · alt+m next model   │
30  ⏵⏵ default   ◆ solo   ▰▱▱▱▱▱▱▱ 14k / 200k                                      claude-sonnet-5-5 ∴ medium   │
```

### Light palette and bypass

The same release drive also used a white tmux window background with
`BINGO_THEME=light`. Unit coverage checks paired chrome tokens, nearest xterm
indices and ground-relative tints while prose stays terminal foreground/SGR dim.
The shift+tab drive observed default → accept edits → plan → bypass → default.
The footer excerpts below retain the native mode words; ANSI captures verified
that bypass uses the bad token in both palettes.

`108x30-dark-bypass`

```text
────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ▍ ask anything
? for shortcuts
 ⏵⏵ bypass   ◆ solo   ▰▱▱▱▱▱▱▱ 13k / 191k                                                  fake-1 ∴ medium
```

`108x30-light-idle`

```text
────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ▍ ask anything
? for shortcuts
 ⏵⏵ default   ◆ solo   ▱▱▱▱▱▱▱▱ ? / ?                                                      fake-1 ∴ medium
```

`108x30-light-bypass`

```text
────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ▍ ask anything
? for shortcuts
 ⏵⏵ bypass   ◆ solo   ▰▱▱▱▱▱▱▱ 13k / 191k                                                  fake-1 ∴ medium
```

```text
release build: Finished release profile [optimized] in 1m 43s; exit 0
108x30-dark: idle/running/done/bypass captured (30 rows each)
80x24-dark: idle/running/done/bypass captured (24 rows each)
108x30-light: idle/running/done/bypass captured (30 rows each)
108x30: rule row 27, prompt row 28, footer row 30
80x24: rule row 21, prompt row 22, footer row 24
ANSI dark: bypass 38;2;224;101;90; rule 38;2;58;52;43
ANSI light: bypass 38;2;176;47;36; rule 38;2;207;197;181
Both palettes: answer prose starts with SGR 39; placeholder uses SGR 2 + 39
Capture assertions: pass; private server/home cleanup: pass
```

Captures waited for the existing eight-second migration notice and permission
mode notices to expire. Those transient hints retain their current placement
until M184. The harness set `exit-empty off` on its private server so closing one
pane could not race the next session; cleanup still killed that server at the end.
