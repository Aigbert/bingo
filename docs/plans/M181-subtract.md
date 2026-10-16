# M181 — Subtract

AGENTS.md "Subtract by default". The person asked for unused logic and easy, low-value tests to go. An audit at 184146db found ~7,900 lines and ~205 of 6,181 tests removable without losing a guarantee; the audit notes (evidence, file:line) are the worker's input, not part of the repo.

## Goal

Delete what nothing uses and tests that guard nothing; make the slow tests cheaper without weakening what they prove. Every contract, consent and fail-closed test survives, as a table where several said the same thing.

## Bricks

1. **Orphaned snapshots.** The seven `.snap` files no test references go. CI's test job runs `cargo insta test --unreferenced=reject` only if `cargo-insta` is already in the workflow; otherwise a discipline check lists `.snap` files whose test name is absent (no new dependency).
2. **Dead public items.** `Executor::execute_one`; sdk `READ_ONLY` and unused test fakes; provider-acp unused method constants and `hang_up`; openai `with_tokens`; `RemoteKernel::into_host`; bingo-update `Settings`/`schema`/ `wanted` and unused re-exports; the stray `crates/bingo-mods/guide.md`; core's never-built `Unreachable` (callers use `Unavailable`); test-only wrappers. Each removal is re-checked with a workspace grep before it goes; agents' `Fleet::post`/`called` stay (mounted by `tests/room_posts.rs`).
3. **Dead manual and ignored tests.** store-jsonl's assert-nothing timing test, `bingo-mods/tests/engine.rs:88`, `crates/bingo/tests/model_picker_tmux.rs`, the TUI storyboard `preview`/`play` and their snapshots, the stale 100 MiB ignored test in surface-rpc.
4. **Tests that restate a constant or manifest.** The manifest / "claims no settings" twins across feature crates; plugin-rpc's copy of bingo-jsonrpc's codec contract; the guide-page shape test copied into ~16 crates becomes one table over registered pages in the binary composition tests. The SDK owns `Page` but cannot import the plugins whose pages must be checked.
5. **Duplicates folded into tables.** TUI `view.rs` snapshots byte-identical to the `screens.rs` catalogue; rooms' five fail-closed tests become one table; store/wake tests repeating bingo-shelf; fs/bash/web duplicates; provider stream-cancel tests that exercise bingo-http go to bingo-http once.
6. **One copy of each helper.** `fnv1a64`, `write_atomically`, the permission mode read, the HTTP error helpers, acp `token.rs` vs bingo-loopback, `instances.rs` logic, test `cx` helpers, the bin's `invalid`/`internal` wrappers. The single copy lives in the lowest crate both sides already depend on; no new crate edge that the layering forbids.
7. **Cheaper slow tests, same property.** Smaller inputs for TUI `view.rs:1672`, layout tests, `blocks.rs:599`, `graphics/band.rs`; the sdk `tree.rs` proptest at 256 cases; a test-injected wake floor instead of waiting the real `WAKE_LEAST`; fewer binary launches inside the skills, checkpoints, plugins and models cli loops; `doctest = false` for libraries with no doctests.
8. **bin test plumbing.** Shared journal helpers in one support module; the provider-environment scrub test compiled into one binary, not ~26; bin test binaries merged where they share setup (target size and link time).
9. **Remove bingo-demo-ui.** Off unless `--demo-ui`; nothing ships it on. The smoke script, `docs/design` tui §8 example, `tests/views.rs` and `cli/plugins.rs` move to bingo-mod-hello first; the `--demo-ui` flag and the `demoUi` key keep parsing (a legacy key, dropped after the next tag) and say the demo is gone. ADR-0001/0013/0057/0069/0081 are amended in place.
10. **SDK event snapshot.** The 552-line event `.snap` is replaced by the record fixtures in `tests/record.rs` once the missing Form fixtures exist.

## Files

Spread across the workspace; the audit notes name each. No production behaviour changes except brick 9's flag message.

## Exit criteria

- [ ] Bricks 1–10 each land as their own commit (or a few), each passing the gates.
- [ ] Test count and summed test time before/after recorded in Verified.
- [ ] `git grep` shows no executable/dependency reference to a removed item; historical records and legacy wire strings are documented exceptions; discipline and budget pass.
- [ ] tui-smoke passes on bingo-mod-hello instead of the demo.
- [ ] Windows all-target check for each touched crate (known C-toolchain deferral recorded).

## Non-goals

Fixing the flaky tests the audit found (tool-bash 150 ms race, channels "nothing happened" sleeps, pty rewind timeout), hooks-shell's hard-coded event list, panel mods that stay open when disabled, auth-oauth's empty `state`: each is a bug fix of its own, listed in Risks for the next round. plugin-rpc `holdsContext` stays until ADR-0076 says otherwise.

## Risks

- A "dead" item used through `#[path]` or a macro: grep the whole workspace, tests included, before each removal.
- A folded table that drops a case: each table lists the cases the removed tests had, one row each.
- Found, not fixed here: the flakes above; hooks-shell `program.rs:21`; five panel mods not closing on disable; auth-oauth `state` disagreement; sdk `is_internal_extension` has no caller.

## Decisions

- Scope is the ten bricks above; optional audit refactors are not extra work.
- Keep the SDK `views` snapshot: `view/tests.rs` explicitly names it. The other seven candidates have no Rust reference, including mounted tests.
- Keep agents' mounted helpers and `is_internal_extension`; enforcing its namespace rule is separate from removing unused implementation.
- Snapshot discipline is advisory and recognizes literal and generated names; CI has no cargo-insta installation, so it adds no dependency.

- Keep fixture-distinct TUI snapshots and `ScheduleShelf`, which still serves the corruption-recovery test. Eleven byte-identical catalogue twins go.

- Mode labels stay an iterator: multiple placed modes are valid, and placements own their strings. First-mode callers retain their existing default.
- Pure error parsing lives in SDK (existing JSON/jiff dependencies); HTTP owns response I/O. Provider classification and credential-boundary tests stay.
- ACP uses the existing loopback token edge and identical comparison/redaction cases; its random-source failure retains the original ACP error message.

- Keep both plugin enable/disable rows: they exercise distinct states. The harness wake floor is one second; normal runs retain ten. Tree sequences retain 80 operations; only the case count drops to 256. Empty rustdoc targets are identified by the baseline; SDK and the hello example stay on.

- Keep journal helpers with distinct result types or read-failure behavior; common projections move to CLI support. Five explicit integration targets retain every mounted scenario and the CI `plugin_kit` target name.

- The binary adds the existing `bingo-mod-hello` workspace edge. Its scripted `Harness` supplies board/progress contributions; ordinary `Hello` keeps its immediate switch contract. Publication is serialized through the SDK token, retaining concurrent edits and late row placement without a mirrored state.
- Legacy demo flags/keys remain parseable with a stderr notice; historical ADR text and persisted demo-name fixtures remain intentionally. No production composition includes the hello harness without both fake-provider switches.

- Keep all 29 event snapshot values as compact `fixtures/record/frames.jsonl` rows, including all three Form cases and the shell item. The record test pins serialization and round-tripping before deleting the old test/snapshot; existing per-variant fixtures remain. Historical milestone references name the old evidence and are not rewritten.

## Verified — 2026-10-16

Commands run in this worktree with disposable HOME/data, provider environment removed, `RUST_TEST_THREADS=2`; ignored/manual tests are never enabled.

```text
Before: RUSTC_BOOTSTRAP=1 cargo test --workspace --locked -- -Z unstable-options --format json --report-time
exit 0; 6,230 passed, 6 ignored; summed per-test time 525.154 s
```

The supplied audit measured 6,226 runs / 1,122.2 s under different load; the fresh baseline above is the comparison for this worktree.

Brick gates select every touched crate with `-p`: `cargo check --all-targets --locked`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`, in that order. Pasted result summaries (exit status):

```text
Brick 1: context, surface-tui — check 0; clippy 0; test 0 (1422 passed, 2 ignored)
Brick 2: core, sdk, provider-acp, provider-openai, surface-rpc, update,
         mods, plugin-host — check 0; clippy 0; test 0
Brick 3: store-jsonl, mods, bin, surface-rpc, surface-tui, pictures
         — check 0; clippy 0; test 0; all six ignored/manual tests removed
Brick 4: bin and all 18 changed page/manifest/codec owners
         — check 0; clippy 0; test 0; all 16 primary page names retained
Brick 5: surface-tui, rooms, schedule, experience, provider-openai,
         provider-anthropic, http, tool-fs, tool-bash, tool-web
         — check 0; clippy 0; test 0; five room tools and five bash endings retained
Brick 6: bin, sdk, core, auth-oauth, http, provider-acp, provider-openai,
         provider-anthropic, permissions, surface-print, surface-rpc, surface-tui
         — check 0; clippy 0; test 0; SDK gates repeated for fixed checksum vectors
Brick 7: workspace (empty rustdoc targets touch most libraries)
         — check 0; clippy 0; test 0; both queued-wake consent cases pass
Brick 8: bin — check 0; clippy 0; test 0 (506 passed)
         inventory 531 -> 506: only 24 scrub-test copies and its one twin removed
Brick 9: bin, mod-hello, mods, surface-tui — check 0; clippy 0; test 0
         legacy CLI/NDJSON 2 passed; RPC lanes/concurrency 4 passed
         isolated hello smoke: 80x24, 120x40, 200x50 passed
Brick 10: sdk — check 0; clippy 0; test 0 (231 passed)
          record contract passed before removal; all 29 JSON values identical
BINGO_UPDATE_SCHEMA=1 cargo test -p <sdk/plugin-rpc/mods> --locked
          all three exit 0; schema/ and sdk/typescript regeneration has no diff
```
