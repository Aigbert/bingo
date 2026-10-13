# M129 — One claim, one shelf

## Goal

ADR-0072 §5–6: finish M109's shared claim by moving the gateway and
store sidecar onto it. Move schedule/experience file stores and ids, and
the three unified-diff copies, into `bingo-shelf`. Keep shelf bytes intact;
modern gateway records add `claimed: true`, with legacy records defaulting false.

## Bricks, in build order

1. **The bare lock.** Unless M109 added it, `bingo-claim` gains
   `lock(path) -> io::Result<Option<File>>`: open with create and no
   truncate, then `try_lock`; `None` means another holder has it.
   `Claim::take` shares its lock primitive after M109's atomic marker
   publication; never publish a blank claim file. Tests: a second handle gets `None`,
   dropping the first frees it, and the file stays.
2. **The store's sidecar.** `bingo-store-jsonl/src/lock.rs` `take` calls
   `bingo_claim::lock` and keeps its `KernelError` mapping. It writes no
   marker, so every existing empty `.lock` stays valid (ADR-0005). The
   lock tests pass unchanged.
3. **Gateway tests first.** Each test uses processes it starts itself.
   Paste today's refusals.
   - A `gateway.pid` that names a live non-gateway (the test's child):
     today `run` refuses; it must start.
   - After the test kills its own gateway child, a new one starts
     without removing anything by hand.
   - A legacy record (no `claimed`) with a live pid still refuses.
4. **The gateway's claim.** `run` takes `bingo_claim::Claim::take(
   data_dir, Protocol { file: "gateway.lock", marker:
   "bingo-gateway-v1\n" })` before it writes the record. `gateway.pid`
   stays the record that `start` waits for and that `status` and `stop`
   read, and gains `claimed: true` (an old record parses as false).
   `pidfile::Claim` and its `create_new` go. `replace` refuses only a
   legacy record with a live pid; a claimed record next to a free claim
   is stale by definition.
5. **Ask the claim who is running.** In `state.rs` `State::read`, a held
   claim is `Running(record)` and a free one with a record is `Stale`;
   the pid probe stays for legacy records only. `stop.rs` signals the
   record's pid only while the claim is held, so a reused pid is never
   signalled. `doctor.rs` `gateway_check` follows `State`, and
   `lock_check` recognises `gateway.lock` as M109 made it recognise a
   channel claim, never offering to remove a modern one.
6. **The shelf's contract.** Create `crates/bingo-shelf`,
   `tier = "library"`, with `serde`, `serde_json`, `ulid`, `similar` and
   `thiserror` (all in tree; budget +1). It holds:
   - `trait Codec { type Record; const EXT; encode; decode }`;
   - `Shelf<C>`: `new(dir)`, `path(id)`, `load() -> Loaded { entries,
     unreadable }` (sorted, why each file is unreadable, dotfiles
     skipped), `save` (via `.{id}.tmp` and rename, temp removed on
     failure), and `delete` (missing is `Ok`);
   - `id::{mint, resolve}`, generic over a key function, so both
     `&[&str]` (schedule) and `&[Entry]` (experience) work;
   - `diff::unified(path, old, new)`.

   Lift schedule's store, id and diff tests first, so the contract is
   tests that already pass.
7. **Schedule moves onto it.** `store.rs` becomes a `Json` codec over
   `Shelf`, and `id.rs` and `diff.rs` go. A golden test: a file written
   before this change loads after it, and saving it again writes the
   same bytes.
8. **Experience moves onto it.** A `Frontmatter` codec; `Library` keeps
   the per-project root; `id.rs` and `diff.rs` go. People edit these
   files by hand, so `load` never rewrites one.
9. **tool-fs takes `diff::unified`.** The library edge
   `bingo-tool-fs → bingo-shelf`; its `diff.rs` and duplicated tests go.

## Files

- `crates/bingo-claim/src/**`, `crates/bingo-shelf/**` (new),
  `Cargo.toml`, `Cargo.lock`, `scripts/budget.toml`
- `crates/bingo-store-jsonl/{Cargo.toml,src/lock.rs}`
- `crates/bingo-gateway/src/{pidfile,run,state,stop,status,doctor}.rs`,
  new integration tests in `crates/bingo-gateway/tests/`
- `crates/bingo-{schedule,experience}/{Cargo.toml,src/**}`: stores,
  codecs, id/diff callers and focused contract tests; leave test-host modules alone
- `crates/bingo-tool-fs/{Cargo.toml,src/{lib,diff,edit,write}.rs}`

## Exit criteria

- [x] Brick 3's three gateway tests pass (paste before and after).
- [x] `rg -n 'try_lock' crates --type rust` finds OS-lock implementation
      only in `bingo-claim` (existing test witnesses and the plugin-rpc
      async mutex are not implementations), and `rg -n 'create_new' crates/bingo-gateway/src`
      finds no exclusion.
- [x] `ls crates/bingo-{schedule,experience}/src/{id,diff}.rs
      crates/bingo-tool-fs/src/diff.rs` finds no file.
- [x] `cargo test -p bingo-claim -p bingo-shelf -p bingo-store-jsonl
      -p bingo-gateway -p bingo-schedule -p bingo-experience -p bingo-tool-fs`
      passes, including the schedule golden bytes.
- [x] `scripts/budget.sh` (+1, `bingo-shelf`), `cargo deny check` and
      `scripts/check_discipline.sh` pass.
- [x] All local gates pass; Windows cross-checks pass for claim, shelf,
      store-jsonl, schedule and experience.
- [ ] Windows CI checks gateway and tool-fs: their `aws-lc-sys` graphs need
      a Windows SDK; deferred by the execution instruction, not attempted here.

## Non-goals

Core's atomic writers (ADR-0072 §7); picture/auth-oauth/update/gateway-log
writers; tool-bash `jobs::mint`; version envelopes or shelf format changes;
gateway detached start; channels' claim (M109).

## Risks

- A pre-M129 gateway holds no `gateway.lock`. The legacy rule covers it,
  and an old binary still refuses a live pid. Name both directions in the
  commit body.
- `gateway.lock` is permanent like the scheduler's; platform termination
  remains the existing probe's responsibility.
- If M122 or M126 moved experience or the gateway's composition, follow
  the tree.
- Re-verified at `fa042aa6`: paths and callers remain; claim has `take`,
  `probe`, `holder`, atomic `publish::open`, but no bare `lock`. Doctor
  already probes schedule/channel claims. `gateway.lock` belongs directly
  under `data_dir`; the existing pid record stays under `gateway/`.
- Per execution instruction, Windows graphs containing `aws-lc-sys` are
  left to Windows CI; check every other touched crate locally.

## Verified (2026-10-13)

Evidence: `target/m129-logs/`. Contract commits precede implementations.

| Command / contract | Before | After |
|---|---|---|
| `cargo test -p bingo-claim --test bare` | `error[E0432]: unresolved import bingo_claim::lock` | `test result: ok. 1 passed; 0 failed` |
| `cargo test -p bingo-gateway --test claim`: claimed record naming live non-gateway | `M129:refused [INVALID_INPUT] a gateway already holds this data dir` | `a_claimed_record_naming_a_live_non_gateway_does_not_block_run ... ok` |
| Same command: killed gateway restart | `a_killed_gateway_can_restart_without_removing_files ... ok` | Same: `ok` |
| Same command: legacy live record | `a_legacy_record_naming_a_live_child_still_refuses ... ok` | Same: `ok` |
| Same command: complete new contract suite | `FAILED. 4 passed; 3 failed` | `test result: ok. 8 passed; 0 failed` |
| `cargo test -p bingo-shelf` | `error[E0432]: unresolved imports bingo_shelf::Codec, bingo_shelf::CodecError, bingo_shelf::Shelf` | store `5 passed`; id `3 passed`; diff `3 passed`; all `0 failed` |
| `cargo test -p bingo-schedule --test shelf` | `pre_m129_schedule_bytes_survive_loading_and_saving ... ok` | Same: `ok` |
| `cargo test -p bingo-experience --test shelf` | `reading_hand_edited_experience_never_rewrites_it ... ok` | Same: `ok` |

- Focused exit-criterion command (`cargo test -p bingo-claim -p bingo-shelf -p bingo-store-jsonl -p bingo-gateway -p bingo-schedule -p bingo-experience -p bingo-tool-fs`): summed output `392 passed; 0 failed`.
- `rg -n 'try_lock' crates --type rust`: implementation at `bingo-claim/src/lib.rs:101`; other matches are existing test witnesses and plugin-rpc's async mutex. `rg -n 'create_new' crates/bingo-gateway/src`: no matches (exit 1). The exit criterion's `ls` command reports `No such file or directory` for all five removed paths (`removed-files.log`).
- `cargo fmt --all -- --check`: exit 0, no output.
- `cargo check --workspace --all-targets --locked`: `Finished dev profile [unoptimized + debuginfo] target(s) in 3.30s`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile [unoptimized + debuginfo] target(s) in 48.50s`. Initial helper-only unwrap lint fixed with the existing integration-test allowance.
- `cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing`: exit 0; summed output `4986 passed; 0 failed; 3 ignored` (existing stress/manual tests). Gateway CLI tests all `ok`; no flaky reruns needed.
- `cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1`: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out`.
- `scripts/check_discipline.sh`: `discipline ok` (historical size warnings).
- `scripts/budget.sh`: `dependencies (unique, normal): 345 (max 345)`; `warm cargo check -p bingo-core: 0s (max 20s)`; `relink isolation: touching the TUI recompiled 0 crates for core`; `target/debug: 4 GB (soft max 5)`; `budget ok`.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok` (existing duplicate-version warnings).
- `cargo tree -p <crate> -i aws-lc-sys --target x86_64-pc-windows-msvc --locked`: no matching package for claim/shelf/store-jsonl/schedule/experience; gateway/tool-fs show `aws-lc-sys v0.44.0` and remain for Windows CI.
- `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`, sequentially: claim `Finished dev profile ... in 0.52s`; shelf `1.70s`; store-jsonl `1.77s`; schedule `1.08s`; experience `4.28s`; each exit 0.
- Token build briefly overlapped the first discipline run; both were rerun sequentially and passed. No terminal-byte changes, so no separate TUI smoke was needed. No push, merge, PR, live configuration change or toolchain install.
