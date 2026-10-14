# M170 — The status hub

ADR-0084, ADR-0086. Needs M167 (placement, `onKey`); prices from M164.
Approved prototype (2026-10-14, "状态 · 插件 · MCP" tab, `/status`
scenarios): https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/status` is one card grown up from the prompt, and every row is a
door: `⏎` on a row runs the command that changes it. Session (rename),
model (`/model`), context with what fills it (compact now), spent (not
selectable), permission mode (`/permission`), each rule this session
accepted (forget it), plugins (`/plugins`), MCP (`/mcp`). Problems are
written on their row in the colour of the problem: a plugin that failed
to load, one from this repository waiting, a server that needs sign-in
or is not running. The feature is the built-in mod `bingo-mod-status`;
the kernel's `/status` key-value view stays as the plain fallback.

## Bricks

1. **What fills the context.** `ContextUsage.parts: Option<[ContextPart; 4]>`
   (additive; `system`, `tools`, `chat`, `pictures`, each tokens):
   `turn.rs::measure` already estimates the system blocks and the
   messages; it keeps the four sums. Fixture; a test that the parts
   sum to the local estimate exactly; server anchoring remains independent.
2. **What it cost.** Pure `spent(usage, price) -> Spent { input, output,
   cache_share, usd: Option<f64> }`; `usd` is `None` where M164's
   catalogue has no price. Table test.
3. **Forget a rule.** `/permission forget <rule>` in `bingo-permissions`
   (a free rule argument; selectable session rules in the card); answers the rule it
   dropped. Test over a session with two rules.
4. **Rows, pure.** `rows(&SessionState, plugins, servers) -> Vec<Row>`
   in the mod: label, value spans, tone (`ok`, `warn`, `bad`, `dim`),
   and the command line `⏎` runs (`None` for spent). Plugins and
   servers are summarised to one row each: counts, then each problem
   by name. Table test per prototype scenario.
5. **Drawing.** `Status` site as a `View` tree: a title line with the
   hint (`live; ⏎ on a row goes there`) and version · uptime, one `❯`
   row, the context bar with its parts on a dim second line, the rules
   indented under permission. At most half the screen; snapshot per
   scenario at 108×30 and 80×24.
6. **Live.** The card redraws from frames while open (a turn finishing,
   a mode change, a server connecting); the cursor stays on its row by
   label, not index. `TestBackend` test: a `ConfigChanged` while open.
7. **Keys.** `↑↓` move over selectable rows, `⏎` runs the row's command
   and closes the card (the next screen opens in its place), `esc`
   closes. Through `onKey` (M167).

## Files

- `crates/bingo-sdk/src/event.rs` (`ContextPart`); `schema/`,
  `sdk/typescript/`; `crates/bingo-core/src/{turn.rs,context/budget.rs}`.
- `crates/bingo-permissions/src/command.rs` (`forget`).
- New `crates/bingo-mod-status/src/{lib.rs,rows.rs,spent.rs,draw.rs,
  keys.rs,dispatch.rs}`; listed in `crates/bingo/src/main.rs`.
- Shared changes: sdk tokens/render/component contracts; MCP services/rows;
  permissions/session; TUI pane drawing/viewport/mouse/tests; bin composition;
  run/pulling and action tests (draw even while actions are queued);
  mechanical ContextUsage literal updates; existing generators and fixtures.
- Reads `catalog(Plugins)` and `bingo_services::mcp::find` (no plugin
  import).

## Exit criteria

- [x] Parts fixture and sum test; spent and rows tables; forget test.
- [x] Snapshots: overview, long chat with two rules, a failed plugin,
      a waiting one, an MCP server needing sign-in, at both sizes.
- [x] `TestBackend`: each row's `⏎` reaches its command; spent is
      skipped by `↑↓`; esc closes; live redraw keeps the row.
- [x] `enabledPlugins: {"bingo.mod.status": false}`: `/status` draws the
      kernel's key-value view.
- [x] tmux drive in a temp HOME against the fake provider, captured and
      compared with the prototype; tui-smoke; native gates as M167.
- [ ] Windows checks for every touched crate (record C-toolchain deferrals).

## Decisions

- The Status site lives in the existing focused, pulled pane above the composer, capped at
  half the viewport. Its keys use the existing pane FIFO and escape path. Empty pane titles
  omit surface chrome so the shared tree owns the single header line. Pane props supply the
  body height to the pull cache. No palette files or new action queue are needed.
- A row command runs through the session handle; its terminal acknowledgement returns as the
  original site key result. Otherwise the TUI ignores the view on an intent it did not submit.
  The instant handler holds no actor queue, so it can await that result while preserving the
  existing surface FIFO.
- Pane/status pulls stay live while commands are queued. Queued dispatch returns immediately;
  stale callback binding is M174's responsibility, not a global redraw guard.
- CommandRun carries the invoking surface; only terminal/desktop invocations open the card.
  Print, channel and non-interactive RPC invocations retain the kernel text command on a shared host.
- StatusProps carry summary/config/context from the asking surface's existing fold. Render
  and arrow handlers never open a session or subscribe; key arguments carry only row targets.
- MCP `Servers::servers` returns connection configuration, not live status.
  Add a default `status` service method and a typed projection of the manager's
  lines in bingo-services/bingo-mcp; no credentials enter the status projection.
- Parts sum to the local estimate, not the anchored server count. Scaling estimates
  to a server count would invent accuracy; the drawing labels them `about`.
- Cost is an estimate at the current model's catalogue rates, explicitly labelled;
  a session may have changed models. Cache writes have no catalogue rate, so cost
  is unknown when they occur, as it is for missing input/output/cache-read rates.
- A Vec removed Copy and required edits in M172-owned mods. The four fixed categories instead
  use an optional array and Copy ContextKind names. JSON remains an additive array; absent
  means unknown. Consumers need only literal updates. Shared token estimation owns the split.
- Command::spec and catalog(Commands) are host-wide; command.describe receives session "host",
  not the asking session. A dynamic rule ArgSpec would list no rules or leak another session's
  rules. Keep a free rule argument; the card supplies exact session rules. Session-aware
  completion belongs to a later catalogue contract, outside M168's protected palette files.
- Command hooks carry no asking surface. Binary composition keeps the plain
  kernel command in print/channel-only runs; terminal and RPC runs install the
  opener. This preserves existing text/NDJSON status behavior without a new
  kernel door or a TUI command special case.
- Guides live in bingo-mod-status/src/guide.md, registered by guide.rs.
- Workspace edges: the new mod uses sdk/services and existing async-trait,
  serde/serde_json/tokio; TUI dev-tests use the mod and existing insta.
  No external dependency is added. Budget counts one new workspace member.

## Non-goals

Editing a value in the card itself (each row goes to its own screen);
a cost history across sessions; the desktop's drawing.

## Risks

- The context split is an estimate and can disagree with the server's
  count: the bar shows `used` from the server, the parts line says
  `about`, and the sum test checks the independent local estimate.
- A summary row that names every failing thing can overflow at 80
  columns: the row truncates names after the first with `+N`.

## Verified — 2026-10-15

Gates ran serially. Provider/proxy environment overrides were unset for local/fake-provider tests and isolated tmux runs; no live-provider or credential tests were added.

| Command | Output summary |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace --all-targets --locked` | Finished `dev` profile; exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Finished `dev` profile; exit 0 |
| `cargo test --workspace --locked` | 172 groups; 5719 passed; 0 failed; 5 ignored |
| `bash scripts/check_discipline.sh` | discipline ok; advisory warnings only |
| `bash scripts/budget.sh` | dependencies 414/414; warm core 0s; relink 0; budget ok; target/debug 8 GB soft-limit warning |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| `bash scripts/tui-smoke.sh` | tui-smoke ok; Counter input persisted at 80/200 columns; question/form cases passed |
| `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked` | Pass: bingo-context, bingo-core, bingo-experience, bingo-mod-status, bingo-persona, bingo-sdk, bingo-services, bingo-skills, bingo-tasks |
| Same Windows command, remaining seven crates | Deferred: bingo, bingo-channel-feishu, bingo-mcp, bingo-plugin-rpc, bingo-surface-print, bingo-surface-tui (`aws-lc-sys`: missing Windows C headers); bingo-permissions (`tree-sitter`/`tree-sitter-bash`: missing stdio.h/stdlib.h) |
| `BINGO_UPDATE_SCHEMA=1 cargo test -p <crate> --locked` generator tests | SDK, plugin-rpc, surface-rpc, store-jsonl and mods generators passed; schema/ and sdk/typescript regenerated |
| `npm test` in sdk/typescript | tests 31; pass 31; fail 0 |
| Status tests and isolated tmux drive | 10 TestBackend snapshots; 3 RPC tests; 7 mod tests; 108x30/80x24 captures compared with prototype; arrows skip spent, Escape closes, Enter displays /rename |

An initial mock CLI test inherited OPENAI_BASE_URL and failed with AUTH_REQUIRED; sanitized runs pass. No auth code changed.
PTY load flakes: `esc_twice_rewinds_the_turn_and_the_file_it_wrote` and `look::a_terminal_whose_ground_turns_light_is_followed_within_one_focus` each failed once, then passed alone in 3.03s/3.12s; the final full suite passes.
The original smoke exposed a stale counter callback during queued input. Review removed the global pull guard: it could freeze first drawings behind queued commands. Handler binding belongs to M174; Status returns a queued acknowledgement immediately. Windows remains unticked because seven checks stop in native C dependencies before Rust checking completes.

Rebase onto `dev` at `c33b0700` — 2026-10-15: replayed all six M170 commits without squashing. Kept both Palette and Status contracts, fixtures, crate edges and registrations; no edited file was deleted. M168's `palette::retired` handling and Plugins refetch remain intact. The combined member budget is 415 (ADR-0086 amended; no external dependency). Initial Clippy found M168's new catalogue-test `ContextUsage` literal missing `parts`; added `None`. Existing SDK/plugin-rpc/surface-rpc/store-jsonl/mods generators ran with `BINGO_UPDATE_SCHEMA=1`: all passed; the subsequent full suite and `git diff --exit-code -- schema/ sdk/typescript/` confirmed no drift. Serial gates: `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0; `cargo test --workspace --locked` with `RUST_TEST_THREADS=4` and OPENAI_API_KEY/OPENAI_BASE_URL/ANTHROPIC_API_KEY/ANTHROPIC_BASE_URL unset: 174 groups, 5686 passed, 0 failed, 5 ignored, no retries; `bash scripts/check_discipline.sh`: discipline ok; `bash scripts/budget.sh`: 415/415 dependencies, warm core 0s, relink 0, budget ok (11 GB target/debug soft warning); `bash scripts/tui-smoke.sh`: tui-smoke ok (223.4s), isolated socket and temporary HOME. `npm test`: 31 passed. Isolated Status drive: 108x30/80x24, navigation, Escape, reopen and Enter-to-rename passed. Existing Windows deferrals above remain unchanged.
