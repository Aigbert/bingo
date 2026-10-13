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

Today the same facts sit in several places:
- `Live.spec`, plus copies in `Live.key`, `Live.cwd` and `Live.parent`.
  The copies are kept in step by hand: `entry.key =
  entry.spec.key.clone()` in `Host::reconfigure`.
- `TurnConfig.session`, a summary snapshot. `restated()`
  (`session.rs`) reads the key and title from it.
- `turn::runs_on` (`turn/config.rs`), which patches provider and model
  onto a summary.
- `resume.rs` `spec_of` reads the summary. `resume_frames` then
  patches the level in from `thinking_of(&replayed)`. That is two
  readers of one fold, with a field left `None` in between.
- `Host::reconfigure` clones the spec, awaits `model_for` and
  `mailbox.summary()`, then writes it back. Two changes at once lose
  one.

Three past fixes were each one field one copy missed: 18e5bc66 (a child
took the host's default model, not the parent's live one), 79107b13
(`/think` lost on resume), e829b872 (resume lost `system_extra` and
`tools`).

## Bricks, in build order

1. **Failing tests first** (`bingo-core/src/host/tests/`):
   - Round trip: open a session from a spec with every field set, and
     again with each optional field `None` and thinking `Some(None)`
     (off). Close it, then check `spec::spec_of(&replayed(journal)) ==`
     the resolved spec. This pins today's behaviour; each of the three
     past fixes would have failed it.
   - Lost update: run `/model` and `/think` on one live session at once
     (`tokio::join!`), using a provider-source fake whose lookup awaits
     a `Notify` (named models bypass the catalog in this tree). Both changes must hold afterwards, in `Live.spec` and
     in the next published summary. Fails today.
   - Resume: a session whose level was set by `/think`, under a
     different settings default, resumes at the `/think` level; one whose
     fold carries no level resumes at the settings default. Pins today's
     order.
   - Messages: after prose, a retry, a compaction, a close and a
     resume, three counts are equal: `store.list`'s summary, the live
     summary, and `replayed(...)`'s fold. This pins today's agreement;
     it should pass unchanged.
2. **One pair.** `bingo-core/src/spec.rs` (new, one noun) holds two
   functions:
   - `stated(spec, choice, became: SessionSummary) -> SessionSummary`
     is the spec as the summary says it, over what the session has
     become: usage, messages, busy, and a title the actor minted.
   - `spec_of(&SessionState) -> SessionSpec` is the inverse. It reads
     the summary, and the level from `state.config` (today's
     `thinking_of`, moved here). A level the fold never carried stays
     absent; the caller fills the settings default.
   `stated` destructures and `spec_of` constructs `SessionSpec` with no `..`, so a new spec field does
   not compile until both say where it goes. The title rule ("a name
   the session already earned is never taken back") lives in `stated`
   only. These sites become calls to the pair:
   - `turn::runs_on`, `restated()`'s body and `resume.rs`'s `spec_of`;
   - the summary patching in `Host::reconfigure` (`summary.key = …`,
     `summary.title = …`).
3. **`Live` holds the spec, not copies of it.**
   - Remove `Live.key`, `Live.cwd` and `Live.parent`. Every read in
     `host.rs` (the key lookups, `check_key_free_except`, the root
     preference `--continue` uses, the parent walks) reads `spec`.
     `created_at` stays; it is not a spec fact.
   - The line `entry.key = entry.spec.key.clone()` is deleted, not
     moved.
4. **The config carries the spec.**
   - `TurnConfig` gains `spec: Arc<SessionSpec>`. `turn_config` builds
     `session` with `stated`.
   - Contributors still read a summary through `ContextQuery.session`.
     That SDK contract stays, and the id readers are unchanged.
   - `restated()` is `spec::stated(&self.config.spec,
     self.config.model.as_ref(), self.state.summary.clone())`.
5. **Changes are serial.**
   - `Live` gains `changing: Arc<tokio::sync::Mutex<()>>`.
     `Host::reconfigure` holds it from reading `live.spec` until it
     writes the spec back and hands the actor its config.
   - The guard is never held while awaiting the session's own mailbox
     from inside that session's actor. Commands run on owned tasks
     (M116), so `/model` calling `reconfigure` cannot deadlock. A test
     runs `/model` as a command.
6. **Resume reads the fold once.** `resume_frames` takes the whole
   spec, level included, from `spec::spec_of(&replayed)`. Its own
   `spec_of` and `thinking_of` are deleted, not kept as fallbacks. The
   settings default fills the level only when the fold carries none.
7. Dated one-line notes on ADR-0008 §4 and ADR-0047 §1 naming
   `spec.rs`.

## Files

- `crates/bingo-core/src/{spec.rs (new),host.rs,host/resume.rs,
  session.rs,turn/config.rs,lib.rs,turn.rs}`, `host/tests/**`
- Mechanical `TurnConfig.spec` fixtures: `src/test_support.rs`,
  `src/turn/tests.rs` in `bingo-core` (protocol step 6).
- `docs/adr/{0008,0047}*.md`

## Exit criteria

- [ ] The lost-update test fails before and passes after (pasted). The
      round-trip, resume and messages tests pass after; the round-trip
      test fails when a scratch edit drops one field from `spec_of`
      (pasted, then reverted).
- [ ] `rg -n 'fn runs_on|entry\.key|summary\.key = ' crates/bingo-core/src`
      is empty (pasted).
- [ ] Adding a field to `SessionSpec` in a scratch edit fails to compile
      in `spec.rs`. Paste the error, then revert.
- [ ] A tmux drive: `/model`, `/think high`, quit, `--continue`. The
      session comes back on that model and level (notes pasted).
- [ ] All gates.

## Non-goals

- Moving the spec into the actor. The host builds configs from its
  registry, and M116 keeps the actor from awaiting that work.
- Changing what `/model` and `/think` write to the settings' user layer
  (79107b13's remembered default stays).
- The tree's copy of summaries (M120).

## Re-verified at `2c5c44ec` (2026-10-13)

- `Live`/`turn_config`/`reconfigure`: `host.rs:115/523/850`;
  `restated`: `session.rs:307`; `runs_on`: `turn/config.rs:66`.
- M114: `resume_frames` folds `store.resume` at `host/resume.rs:76`;
  `thinking_of`/`spec_of` remain at 105/183. Keep checkpoint replay.
- M116: `session/commands.rs:260` spawns owned command tasks. M131
  chains and M119 services need no changes; M115 storage is unchanged.
- All three cited fixes remain: `inherited`, resume config-view effort,
  and summary `system_extra`/`tools`; their regression tests still exist.
- Module wiring (`lib.rs`, `turn.rs`) and two config fixtures were missing
  from Files. No SDK contract, dependency or ADR Decision changes.

## Risks

- `ContextQuery.session` is a plugin-visible summary. Its contents must
  not change; assert this with the existing contributor tests. No SDK
  type changes in this milestone; if one seems needed, stop and ask.
- The guard is per session. A change that touches a parent and a child
  (none today) would need an order. Note it in `spec.rs`.
- Re-verify at start: M116 has landed (commands on owned tasks); the
  names `restated`, `runs_on`, `spec_of`, `thinking_of`, `Live`,
  `Host::reconfigure`.
