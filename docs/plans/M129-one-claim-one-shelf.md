# M129 — One claim, one shelf

## Goal

ADR-0072 §5–6. M109 created `bingo-claim` and moved the scheduler and
the channels onto it. This milestone moves the last two lock styles:

- **The gateway.** Its pid file is the exclusion today, so a pid reused
  after a kill wedges every later start (`run.rs` `replace` never steps
  on a live pid "whatever it turns out to be running").
- **The store's sidecar.** It is already an OS lock; only the code
  moves.

After this, `try_lock` appears in one crate. The schedule and
experience stores, their `id.rs`, and the three `diff.rs` copies become
`bingo-shelf`. No format on disk changes.

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

- [ ] Brick 3's three gateway tests pass (paste before and after).
- [ ] `rg -n 'try_lock' crates --type rust` finds OS-lock implementation
      only in `bingo-claim` (existing test witnesses and the plugin-rpc
      async mutex are not implementations), and `rg -n 'create_new' crates/bingo-gateway/src`
      finds no exclusion.
- [ ] `ls crates/bingo-{schedule,experience}/src/{id,diff}.rs
      crates/bingo-tool-fs/src/diff.rs` finds no file.
- [ ] `cargo test -p bingo-claim -p bingo-shelf -p bingo-store-jsonl
      -p bingo-gateway -p bingo-schedule -p bingo-experience -p bingo-tool-fs`
      passes, including the schedule golden bytes.
- [ ] `scripts/budget.sh` (+1, `bingo-shelf`), `cargo deny check` and
      `scripts/check_discipline.sh` pass.
- [ ] Windows `cargo check` passes for claim, shelf, gateway and
      store-jsonl; all gates pass.

## Non-goals

- Core's atomic writers (ADR-0072 §7). The writers in pictures,
  auth-oauth, update and the gateway log adopt the shelf later, one at a
  time if at all. tool-bash's `jobs::mint` (the same slug with a prefix)
  likewise.
- Any on-disk format change. In particular, no version envelope: a newer
  file an older binary cannot parse is already listed as unreadable.
- The gateway's detached start; the channels' claim (M109).

## Risks

- A pre-M129 gateway holds no `gateway.lock`. The legacy rule covers it,
  and an old binary still refuses a live pid. Name both directions in the
  commit body.
- On Windows an open file cannot be deleted, so `gateway.lock` is
  permanent like the scheduler's; `stop` keeps its Windows kill.
- If M122 or M126 moved experience or the gateway's composition, follow
  the tree.
- Re-verified at `fa042aa6`: paths and callers remain; claim has `take`,
  `probe`, `holder`, atomic `publish::open`, but no bare `lock`. Doctor
  already probes schedule/channel claims. `gateway.lock` belongs directly
  under `data_dir`; the existing pid record stays under `gateway/`.
- Per execution instruction, Windows graphs containing `aws-lc-sys` are
  left to Windows CI; check every other touched crate locally.
