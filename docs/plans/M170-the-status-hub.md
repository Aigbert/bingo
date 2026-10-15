# M170 — The status hub

ADR-0084, ADR-0086; M167 supplies placement/onKey; M164 supplies prices.
Approved prototype: 2026-10-14, "状态 · 插件 · MCP", `/status` scenarios.
https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/status` opens one live card above the composer. Rows lead to session rename,
model choice, compaction, permission mode/rule removal, plugins and MCP. Spent
is informational. Problems appear on their own rows. The feature is the built-in
`bingo-mod-status`; non-interactive callers retain the kernel's text table.

## Bricks

1. `ContextUsage.parts: Option<[ContextPart; 4]>`: system/tools/chat/pictures.
   Shared estimation partitions local tokens; fixture and independent sum test.
2. Pure `spent(usage, price)`: charge each used component with a known rate;
   missing rates for used components make the cost unknown. Table tests.
3. `/permission forget <rule>` drops one session rule. Two-rule regression.
4. Pure rows from Status facts, plugin catalogue and live MCP service status.
   Typed states determine counts and problem tones; prototype scenario tables.
5. Status View tree: heading, version/uptime, selected row, context bar/parts,
   indented rules, problem colours. Snapshots at 108×30 and 80×24.
6. Live invalidation from registered event hook and 1 Hz service/uptime refresh.
   Keep selection by row ID; test real Invalidate calls and lease recovery.
7. ↑↓ skip spent; Enter closes and runs the selected action; session/permission
   prefill editing. Idle Escape closes; busy Status Escape ends the turn.

## Files

- sdk event/origin, catalogue command payload, component/render/slot contracts;
  schema/ and sdk/typescript through existing generators.
- core turn/measure.rs, context/budget.rs, model catalogue/snapshot and commands.
- permissions command/session; services/MCP status projection.
- new bingo-mod-status: rows/spent/draw/keys/dispatch/live/command and guide page.
- TUI pulled panes, surface actions, origins and tests; RPC caller projection;
  binary composition and black-box tests; mechanical Origin/ContextUsage literals.
- scripts/models_dev.sh preserves cache-write rates; no added external dependency.

## Exit criteria

- [x] Context fixture/local sum, cost/state tables, embedded prices and forget tests.
- [x] Prototype scenarios at both sizes, including rules and plugin/MCP problems.
- [x] Keys skip spent, dispatch/prefill, preserve drafts, Escape and live selection.
- [x] Disabled mod and non-interactive callers retain the kernel text table.
- [x] Empty pull props draw; expired selection resumes; client commands are ignored.
- [x] Final native gates and isolated tmux drive for fix round 2.
- [ ] Windows checks for every touched crate (native C-toolchain deferrals below).

## Decisions

- Status uses the existing focused pulled pane above the composer, capped at half
  the viewport. Empty pane titles avoid duplicate chrome. Body height enters pull
  props. Existing pane FIFO/site keys handle input; no additional ordering queue.
- Ordinary panes keep dev's Escape behavior. Status follows the existing site
  convention: idle closes, busy ends the turn. Question/form paths stay intact.
- One typed Composer action, `composer::fill::Fill`, serves the hub's `/rename `
  and `/permission ` prefill and browse rewind: a nonempty draft is kept with a
  notice, never overwritten; M177's `browse::composer` was folded into it.
- Row dispatch uses the session handle and returns its first acknowledgement to
  the original key action. Queued returns its position immediately; the instant
  handler holds no actor queue. Pulls continue while actions are queued. Stale
  callback binding belongs to M174, not a global pull guard.
- `Acknowledged` forwards the entire existing result (view/message/surface/item/
  answer) without executing or recording it again.
- `Origin.render_surface` and `CommandRun.surface` use existing `SurfaceName`.
  Attribution IDs still match client IDs for queue ownership; renaming TUI's ID
  cannot change its separately typed interaction family.
  RPC derives the type from its declared client when input has none. Terminal and
  desktop callers get the card; plain RPC/print/channels retain the text command.
- Status props optionally carry summary/config/context from the caller's fold,
  without transcript bodies. Empty props use the host's 64 KiB-bounded attachment;
  requiring client-filled facts broke generic desktop/RPC pull clients (fix 2).
- Key actions carry only a row ID, never a command/list of commands. The host
  re-derives rows from a bounded attachment before navigation or Enter; unknown
  IDs are no-ops. This replaces trusting client rows and the old no-read arrow path.
- Open cards have a five-second render/key lease; unviewed entries stop ticking.
  A render can restore its supplied selection and renew the entry after expiry.
  TUI forwards selection from its displayed tree. The contract has no viewer ID,
  so simultaneous viewers still share a session cursor. Refresh hooks resume too.
- Plugin states deserialize as SDK PluginState; malformed states say unavailable.
  Registering is pending; Pending counts its current state and warns of transition.
  MCP rows read M173's `ServerRow` from `Servers::rows`; `Status::said()` in
  services/mcp/row.rs is the one wording both panels use (rebase onto af2a1cde).
- Context parts sum to the local estimate, not the anchored server count; scaling
  would invent accuracy. Label them `about`. Four fixed categories use an optional
  array and Copy names, avoiding a Vec-induced loss of Copy in shared consumers.
- Cost is an estimate at the current model's catalogue rates, not historical bills.
  Only used components without rates make it unknown. Preserve cache-write rates
  in parser/pruner and regenerate the embedded snapshot with --preserve-capabilities;
  never hand-edit prices or change existing request capabilities during refresh.
- Command specs/catalogues are host-wide and command.describe uses session "host".
  Keep a free rule argument: dynamic completion would hide or leak session rules.
  The card provides exact rules; session-aware catalogue completion is out of scope.
- M168 palette retirement and Plugins refetch remain; M172 live states are preserved.
  Measurement extraction uses free functions to respect size/impl-spread limits.
- Guides live in bingo-mod-status/src/guide.md, registered as a Page in guide.rs.
  No docs/guide directory. Desktop drawing and in-card value editing are out of scope.
- Workspace edges use sdk/services and existing async-trait/serde/serde_json/tokio;
  TUI tests use the mod and existing insta. Combined normal-tree budget is 415.

## Non-goals and risks

No historical cost ledger or desktop renderer. Estimates can differ from server
counts. At 80 columns, long problem lists show the first name and `+N`. No viewer
identity means shared cursors. Missing native C toolchains limit Windows evidence.

## Verified — 2026-10-15

Earlier runs (initial 5719, c33b0700 5686, 222d4c72 5749/5757 tests) passed all
native gates, schema drift checks, 31 TypeScript tests and isolated Status drives.
Windows: core/context/experience/mod-status/persona/sdk/services/skills/tasks
passed; bin/channel-feishu/mcp/plugin-rpc/print/tui stop in aws-lc-sys headers,
permissions in tree-sitter C headers, so Windows stays unticked. Fix round 2
regenerated prices with `models_dev.sh --preserve-capabilities` (generator-only).

Final gates ran serially with `RUST_TEST_THREADS=4`; OPENAI_API_KEY/OPENAI_BASE_URL/ANTHROPIC_API_KEY/ANTHROPIC_BASE_URL and provider/proxy overrides unset.

| Command | Output summary |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace --all-targets --locked` | Finished `dev` profile; exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Finished `dev` profile; exit 0 |
| `cargo test --workspace --locked` | 174 groups; 5757 passed; 0 failed; 5 ignored; final run 192.0s |
| `bash scripts/check_discipline.sh` | discipline ok; advisory warnings only |
| `bash scripts/budget.sh` | dependencies 415/415; warm core 0s; relink 0; budget ok; 17 GB target/debug soft warning |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| `bash scripts/tui-smoke.sh` | tui-smoke ok; final run 221.8s; isolated socket/temp HOME; counter and question/form cases passed |
| `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked` | Passed: agents, core, mod-status, rooms, schedule, sdk, surface-rpc |
| Same Windows command | Deferred: bingo, channels, surface-tui; aws-lc-sys missing Windows C headers (`stdlib.h`); criterion stays unticked |
| `BINGO_UPDATE_SCHEMA=1 cargo test -p <crate> --locked` generators | SDK/plugin-rpc/surface-rpc/store-jsonl/mods passed after baseline regeneration; `git diff --exit-code -- schema/ sdk/typescript/` exit 0 |
| `npm test` in sdk/typescript | tests 31; pass 31; fail 0 |
| Focused tests and isolated Status tmux drive | 18 mod tests; 1160 TUI passed/2 ignored; 6 RPC tests; 108x30/80x24, navigation, Escape, reopen and both prefills passed |

Rebase onto dev af2a1cde: fmt/clippy 0; tests 185 groups, 5991 passed, 0 failed,
5 ignored; schema regen clean; 32 TS tests; discipline/budget/deny ok; smoke 3/3;
tmux drive 108x30/80x24: hub and /plugins both show repo waiting for your yes.
