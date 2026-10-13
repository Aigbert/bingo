# M118 — The session spec has one owner

## Goal

A session's spec is what it was asked to be: cwd, key, parent, title,
driver, provider, model, `system_extra`, tools and thinking. It gets
one live holder, the host's `Live.spec`. It keeps one durable form, the
journal's fold: the `SessionSummary`, and for the level the config view
(ADR-0008 §4 makes `ConfigChanged` the one place it is read). One pair
of exhaustive functions joins the two, with a round-trip test. No ADR:
ADR-0008 §4 and ADR-0047 §1 already say the spec is "the one holder".
This makes it true.

The duplicated key/cwd/parent in `Live`, summary patches in `restated`,
`runs_on` and `reconfigure`, and separate resume readers are replaced.
Concurrent `reconfigure` calls currently clone before awaits and lose updates.
Past fixes to preserve: 18e5bc66 (inherit the parent's live model), 79107b13
(keep `/think` on resume), e829b872 (keep `system_extra` and `tools`).

## Bricks, in build order

1. **Failing tests first** (`host/tests/`): round-trip every spec field,
   then each optional field absent and thinking explicitly off; close and
   compare `spec_of(replayed(journal))` with the resolved spec. Pin the
   three past fixes. Concurrent Model/Thinking changes (the commands' verb)
   use `tokio::join!` and a provider-source lookup held by `Notify`; assert
   both in `Live.spec` and the published summary/config. Resume prefers the
   `/think` level over a different settings default; an absent level takes
   that default. After prose, retry, compaction, close and resume, the
   store listing, live summary and replay fold agree on message counts.
2. **One pair**, in new `bingo-core/src/spec.rs`:
   `stated(spec, choice, became: SessionSummary) -> SessionSummary` overlays
   the spec on progress (usage/messages/busy and an actor-minted title);
   `spec_of(&SessionState) -> SessionSpec` reads summary and config level.
   The former destructures, the latter constructs `SessionSpec` exhaustively,
   without `..`, so a new field cannot compile until both handle it.
   `stated` alone preserves an earned title when the spec has none. Replace
   `runs_on`, `restated`'s merging, resume's `spec_of` and reconfigure's
   key/title patching with this pair. Absent effort stays absent for defaults.
3. **Live holds no copies**: remove `Live.key`, `cwd`, `parent`; all key
   lookup, key exclusion, latest-root selection and parent walks read `spec`.
   `created_at` stays. Delete `entry.key = entry.spec.key.clone()`.
4. **The config carries the spec**: add `TurnConfig.spec: Arc<SessionSpec>`;
   `turn_config` builds `session` with `stated`, and `restated` uses spec,
   choice and the current state summary (retaining its timestamp update).
   Contributors keep `ContextQuery.session`; SDK contract and id readers stay.
5. **Changes are serial**: `Live.changing: Arc<tokio::sync::Mutex<()>>` is
   held from reading the current spec through writing it and handing the
   actor its config. Never await that actor's mailbox from inside the actor
   while holding the guard. M116's owned commands allow `/model` to await
   reconfiguration; exercise the real command path in its existing test.
6. **Resume reads once**: take the whole spec from `spec_of(&replayed)`,
   deleting resume's `spec_of` and `thinking_of`. Fill the settings default
   only when the fold carries no effort. Preserve M114's checkpoint path.
7. Dated notes on ADR-0008 §4 and ADR-0047 §1 naming `spec.rs`.

## Files

- `crates/bingo-core/src/{spec.rs (new),host.rs,host/resume.rs,
  session.rs,turn/config.rs,lib.rs,turn.rs,host/tests.rs}`, `host/tests/**`
- Mechanical `TurnConfig.spec` fixtures: `src/test_support.rs`,
  `src/turn/tests.rs`, `src/session/tests/{log,images}.rs` in
  `bingo-core` (protocol step 6; the latter construct Log configs).
- `docs/adr/{0008,0047}*.md`

## Exit criteria

- [x] The lost-update test fails before and passes after (pasted). The
      round-trip, resume and messages tests pass after; the round-trip
      test fails when a scratch edit drops one field from `spec_of`
      (pasted, then reverted).
- [x] `rg -n 'fn runs_on|entry\.key|summary\.key = ' crates/bingo-core/src`
      is empty (pasted).
- [x] Adding a field to `SessionSpec` in a scratch edit fails to compile
      in `spec.rs`. Paste the error, then revert.
- [x] A tmux drive: `/model`, `/think high`, quit, `--continue`. The
      session comes back on that model and level (notes pasted).
- [x] All gates.

## Non-goals

- Moving the spec into the actor. The host builds configs from its
  registry, and M116 keeps the actor from awaiting that work.
- Changing what `/model` and `/think` write to the settings' user layer
  (79107b13's remembered default stays).
- The tree's copy of summaries (M120).

## Re-verified at `2c5c44ec` (2026-10-13)

- `Live`/`turn_config`/`reconfigure`: `host.rs:115/523/850`;
  `restated`: `session.rs:307`; `runs_on`: `turn/config.rs:66`.
- M114: resume folds `store.resume` at `host/resume.rs:76`;
  `thinking_of`/`spec_of`: 105/183. Keep checkpoint replay.
- M116 commands run on owned tasks (`session/commands.rs:260`). The three
  past fixes remain in inheritance and resume; M131/M119/M115 need no edits.
- Named models bypass the catalog; pause provider-source lookup instead.
  Module wiring and config fixtures were missing from Files; no Decision changes.

## Risks

- `ContextQuery.session` is a plugin-visible summary. Its contents must
  not change; assert this with the existing contributor tests. No SDK
  type changes in this milestone; if one seems needed, stop and ask.
- The guard is per session. A change that touches a parent and a child
  (none today) would need an order. Note it in `spec.rs`.
- Re-verify at start: M116 has landed (commands on owned tasks); the
  names `restated`, `runs_on`, `spec_of`, `thinking_of`, `Live`,
  `Host::reconfigure`.

## Verified (2026-10-13)

All gates ran sequentially; none remain unmet. Logs/captures: `target/m118-verification/` (ignored).

```text
Before: cargo test -p bingo-core host::tests::spec_owner --locked
  concurrent_model_and_thinking_changes_keep_both ... FAILED
  left: Some(None); right: Some(Some(High)); 3 passed; 1 failed
After: cargo test -p bingo-core --locked
  test result: ok. 468 passed; 0 failed
  All four spec_owner regressions also passed in the final workspace run.
Scratch: cargo test -p bingo-core every_spec_field_survives_the_journal_round_trip --locked
  FAILED: assertion left == right; tools: None versus Some(["Echo"])
Scratch: cargo check -p bingo-core --lib --locked
  spec.rs:15 error[E0027]: pattern does not mention field `m118_scratch`
  spec.rs:46 error[E0063]: missing field `m118_scratch` in initializer
  Both scratch edits reverted; no SDK changes remain.
rg -n 'fn runs_on|entry\.key|summary\.key = ' crates/bingo-core/src
  no output (exit 1: no matches)
cargo fmt --all -- --check
  exit 0
cargo check --workspace --all-targets --locked
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 59s
cargo clippy --workspace --all-targets --locked -- -D warnings
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 12s
cargo test --workspace --locked -- --test-threads=1 --skip a_token_may_be_dialled_again_once_its_stream_has_closed --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing
  exit 0; aggregate: 5149 passed; 0 failed; 4 existing ignored; 2 filtered out
cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1
  test result: ok. 1 passed; 0 failed
cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
  test result: ok. 1 passed; 0 failed (5151 total); all 16 PTY tests passed
scripts/check_discipline.sh: discipline ok
scripts/budget.sh: dependencies 352 (max 352); warm core 2s; isolation 0; target 5 GB; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
cargo check -p bingo-core --all-targets --locked --target x86_64-pc-windows-msvc
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 14s; no CI deferral
scripts/tui-smoke.sh (private TMUX_TMPDIR): tui-smoke ok
python3 target/m118-verification/resume-drive.py
  /model fake/m118-model; /think high; quit 0; --continue; model fake/m118-model, thinking high; quit 0
  Same session, despite changed defaults low/fake-default. Temporary homes/sockets removed.
```
