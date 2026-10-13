# M135 — One lifecycle vocabulary

## Goal

ADR-0077: one pure decision in `bingo-collab` speaks `working`, `asking`,
`idle`, `stored`, `not seated` to agents tools, `/team`, and the TUI.
`SessionSummary.asking` lets listings see an open prompt without a child fold.

## Bricks, in build order

1. Contract first: defaulted `SessionSummary.asking`, legacy fixture, failing
   permission open/answer/esc tests, and the five lifecycle words.
2. Pure `decide(Facts { seated, log, open, asking, working })`, precedence
   and ties; `Lifecycle::of_state` reads interactions/turn, `of_summary` stamps.
3. Actor `stamped()` reads `pending`; open, answer and cancellation republish
   the summary. Complete summary constructors initialize `asking: false`.
4. Agents listing/tone, `/team`, guide and stop/dismiss use the shared type.
5. Pure TUI `said(lifecycle, last, spent)`: verdict follows `idle`; a never-run
   child's brief stays beside it. Keep attention, ears and unread independent.
6. Replace TUI `Status`; `Wants::Running` reads `Working`; preserve colors; renew snapshots separately.
7. Accept ADR-0077 and annotate ADR-0027, ADR-0029 and ADR-0060.

## Files

- SDK `src/event.rs`, compatibility/record/checkpoint fixtures, snapshots,
  `src/state.rs`, `src/store/checkpoint/tests.rs`, `tests/record.rs`.
- Core `src/session.rs` (stamp only), `session/interactions.rs`,
  `session/tests.rs`, `session/tests/asking.rs`.
- Collab `src/{lib,lifecycle}.rs`; agents `src/{list,stop,dismiss,tests}.rs`,
  `src/guide.md`, `src/team/command.rs`.
- TUI `src/{lib,lifecycle,tree,roster,seats,transcript,run}.rs` and screen tests.
  `views/tree.rs` stays a generic tree renderer; no new dependency is needed.
- Mechanical summary constructors across the workspace (protocol step 6).
  Mechanical vocabulary fixtures: agents `command.rs`; TUI `motion.rs`,
  `view.rs`, `screens/teams.rs` and snapshots; `scripts/tui-smoke.sh`.
- JSONL `journal.rs`/`fixtures/clean-v3.jsonl` (v1 kept); generated schemas and `sdk/typescript/src/wire.ts`.
- ADR-0077 and dated notes in ADR-0027, ADR-0029, ADR-0060.

## Exit criteria

- [x] Lifecycle word/precedence/reader tests pass.
- [x] Core permission open stamps true; answer and esc publish false.
- [x] Agents list asking, refuse dismissal, and show an empty role as not seated.
- [x] Retired lifecycle-word search reviewed; remaining hits explained below.
- [x] TestBackend rows, PTY smoke and protocol gates pass, with step 7 deferrals.

## Non-goals

No change to `busy`, channels, room ears, attention/unread, window notifications, hook/deadline waits, dependencies, or M136.

## Risks

- Asking between turns: StopAgent refuses in words; the person answers/cancels
  the prompt. Pinned by a test and stated in the guide, as the original risk allowed.
- Two summary publications per prompt: TreeState applies to one keyed state,
  without re-folding or traversing the tree. No surface mirror was added.
- Old tool output stays historical; new descriptions teach the shared words. MSVC deferrals follow step 7.

## Re-verified (2026-10-13, f9eb788a)

Committed before code as `860f5476`. M108–M125 and M127–M134 are present.
M120's fold and M123's collab seats remain the readers; M125 focus/render need
no change. M134's Provider/Agent permissions reach the same pending table.
Only the stamp changes in `session.rs`; M136's hook work remains independent.
All interaction cancellations converge on `cancel_each`; no expiry scheduler
exists. M113's old summary is retained separately to pin the default.

## Verified (2026-10-14)

Contracts failed before implementation and were committed separately:
```text
cargo test -p bingo-core asking:: --locked
an open permission must stamp asking
test result: FAILED. 0 passed; 2 failed
cargo test -p bingo-collab lifecycle:: --locked
test result: FAILED. 1 passed; 1 failed
cargo test -p bingo-agents an_open_prompt --locked
test result: FAILED. 0 passed; 2 failed
cargo test -p bingo-surface-tui lifecycle:: --locked
test result: FAILED. 0 passed; 1 failed
```
Initial failures: clippy caught an orphaned NOT_SEATED comment; the JSONL fixture lacked `asking: false`.
Both were repaired within scope. BINGO_UPDATE_SCHEMA=1 regenerated schemas,
wire.ts and current fixtures; INSTA_UPDATE=always renewed snapshots. Normal
workspace tests below subsequently checked them without update switches.
```text
cargo fmt --all -- --check
fmt exit: 0
cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.62s
cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.70s
cargo test --workspace --locked -- --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed
test session::tests::asking::an_answer_republishes_not_asking ... ok
test session::tests::asking::esc_republishes_not_asking ... ok
test lifecycle::tests::readers_use_their_own_source_of_truth ... ok
test dismiss::tests::an_open_prompt_cannot_be_dismissed ... ok
test list::tests::an_open_prompt_lists_as_asking ... ok
test result: ok. 1124 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 2.11s
cargo test -p bingo-provider-acp --test bridge token --locked -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.01s
scripts/check_discipline.sh
discipline ok
scripts/budget.sh
dependencies (unique, normal): 355 (max  355)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 5 GB (soft max  5)
budget ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
bash /tmp/m135-tui-smoke.sh
tui-smoke ok
```
Workspace total: 5,158 passed, zero failed, four pre-existing ignored
measurement/stress/storyboard tests; plus the three token tests run alone.
All 16 PTY tests passed; no flaky retry. Smoke used the repository script with
only ROOT fixed and a private `bingo-m135-11ea55c182c7` socket; its cleanup ran.
No live or credential tests, push, merge, or PR. No unresolved stop condition.
Each following crate ran `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc` sequentially:
```text
bingo-agents: exit=0
bingo-channel-adapter: deferred (aws-lc-sys; Windows SDK C headers unavailable)
bingo-checkpoints: exit=0
bingo-collab: exit=0
bingo-context: exit=0
bingo-core: exit=0
bingo-demo-ui: exit=0
bingo-experience: exit=0
bingo-persona: exit=0
bingo-plugin-rpc: deferred (aws-lc-sys; Windows SDK C headers unavailable)
bingo-rooms: exit=0
bingo-sdk: exit=0
bingo-skills: exit=0
bingo-store-jsonl: exit=0
bingo-surface-print: deferred (aws-lc-sys; Windows SDK C headers unavailable)
bingo-surface-rpc: exit=0
bingo-surface-tui: deferred (aws-lc-sys; Windows SDK C headers unavailable)
bingo-tasks: exit=0
```
The four deferrals fail only in `aws-lc-sys v0.44.0`: `fatal error: 'stdlib.h' file not found` / `'windows.h' file not found`.
No Rust compile errors; these four checks remain for Windows CI.

`rg -n '"busy"|"running"|"done"|Needs you|Starting'` over agents/TUI `src`,
excluding `**/tests/**`, has no lifecycle label. Remaining hits: agents
`spawn.rs` comment and `watch.rs` answer fixtures; TUI `view.rs`/`status.rs`
negative assertions; `views/tests.rs` generic badge, `thoughts.rs`/`fold.rs`
answer fixtures, `screens/windows.rs` task fixture, `rail.rs` demo signal;
`terminal.rs` and `run.rs` are the unchanged turn-completion notification/test.
