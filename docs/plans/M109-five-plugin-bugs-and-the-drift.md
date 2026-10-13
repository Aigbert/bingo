# M109 — Five plugin bugs and the drift

## Goal

Fix the plugin-side bugs audit-2026-10 found ("Bugs that do not wait")
and the records that disagree with the tree. Each fix is small and
independent; one commit each. One of them creates a library crate: the
claim protocol M106 wrote for the scheduler becomes `bingo-claim`, so
the channels' claim stops going stale. That is ADR-0072's claim
decision taken early (its status line says so).

## Bricks, in build order

1. **`bingo-claim`: the one persistent claim.**
   - New crate `crates/bingo-claim`, `[package.metadata.bingo] tier =
     "library"`, depending on nothing but std and `thiserror`.
   - Lift `bingo-schedule/src/lock.rs` with `lock/publish.rs` and
     `lock/tests.rs`: `Claim::take`, `probe`, `holder`, `ClaimError {
     WouldBlock, Legacy, Io }`, the staging-file-plus-hard-link publish.
     Parameterise by a `Protocol { file: &str, marker: &str }`; schedule
     passes `runner.lock` and its existing `MARKER` byte-for-byte, so its
     files on disk do not change. Replace the publisher's `ulid` staging
     name with a PID/counter `create_new` retry, keeping std-only code.
   - Tests move with the code; schedule keeps one test that its protocol
     constants are unchanged.
2. **Channels move onto it.** `bingo-channels/src/lock.rs` `Claim::take`
   uses `File::create_new` + pid + remove-on-drop, so a killed process
   leaves a file that refuses every later start. Replace it with a
   `bingo_claim::Claim` under protocol
   `{ file: "<slug>.lock", marker: "bingo-channel-claim-v1\n" }` in the
   same directory. A legacy file (a bare pid, or empty) is refused
   without mutation, with the remedy (stop every bingo using this data
   dir, remove that file); `held_elsewhere` keeps answering from the one
   refusal text. Contract tests first, as M106 did: a second process is
   refused; after the first is killed, a new one takes the claim with no
   file removed.
3. **Doctor knows both.** `bingo-gateway/src/doctor.rs` probes a channel
   lock through `bingo_claim::probe` (as `schedule_lock` does today),
   never offers to remove a modern one, and keeps its legacy advice for
   a pid file.
4. **Agent names compare as rooms compare.** `bingo-agents/src/names.rs`
   compares exactly (`free`: `held == name`; `check`: `name == PARENT`),
   `bingo-rooms/src/name.rs` `same` compares case-insensitively, so
   `Reviewer` and `reviewer` are two agents and one room member. Agents
   adopts the rooms rule: `free`, `check` (so `Parent` is refused) and
   every lookup in `names.rs` (`resolve`, `agent`, `addressed`) go
   through a local `same` with the same body. A test in `crates/bingo`
   (which may depend on both) runs one table of pairs through both
   `same`s and asserts they agree. M122 merges them (ADR-0071).
5. **The ACP agent sees pictures.** `bingo-provider-acp/src/provider.rs`
   `said` keeps text parts only. When the agent's
   `prompt_capabilities.image` is set, send each `ContentPart::Image` as
   an ACP image content block in order; otherwise send the text and one
   line per picture, `[picture not sent: <media type> <size>; this agent
   does not take pictures]` (size via `bingo_sdk::bytes::words`). Test
   both with the fake ACP agent.
6. **A plugin's tool output is bounded like any other.**
   `bingo-plugin-rpc/src/tool.rs` `output` passes the plugin's parts
   through unchanged; ADR-0062 §2 says every producer goes through
   `bingo_pictures::bounded`. Map each `ContentPart::Image` through it
   (on a blocking thread, M61's rule); a picture that cannot be bounded
   becomes a text part naming why. Adds the library edge
   `bingo-plugin-rpc → bingo-pictures` (ADR-0012 §1 allows it).
7. **The records say what the tree is.**
   - ADR-0001's crate list names `bingo-teams`, `bingo-acp` and
     `bingo-services`, none of which exist: add a dated note with the
     current list from `ls crates` and the tiers.
   - ADR-0004 §4 says "six kernel keys": count ADR-0003 §2's list as it
     stands and note the number, or replace the number with the name.
   - ADR-0035 §7 describes a `bingo-acp` surface crate: dated note on
     what exists (`bingo-provider-acp` and its bridge).
   - `crates/bingo/src/env.rs` says "twenty-one" variables: remove the
     count (a count in a comment is a second copy of the list; the
     module itself says it is the rule, not a list).
   - `bingo-provider-openai/src/instances.rs` `BUILT_IN` spells
     `anthropic` and `fake`, other plugins' ids. Keep its own two; the
     registry already refuses a second provider of a name (the comment
     above it says so). A test: an instance named `anthropic` still
     fails the boot, naming the collision.

## Files

- `crates/bingo-claim/**` (new), `Cargo.toml` (workspace dependency),
  `Cargo.lock`, `scripts/budget.toml` (+1 member; no new external crate)
- `crates/bingo-schedule/{Cargo.toml,src/lock.rs,src/lock/*}` → wrappers
- `crates/bingo-channels/{Cargo.toml,src/lock.rs,src/host.rs,src/guide.md}`
- `crates/bingo-gateway/{Cargo.toml,src/doctor.rs}` and tests
- `crates/bingo-agents/src/{names.rs,lib.rs}` and tests; `crates/bingo/tests/`
- `crates/bingo-rooms/src/lib.rs` (export the existing name comparison)
- `crates/bingo-provider-acp/src/provider.rs` and tests
- `crates/bingo-plugin-rpc/{Cargo.toml,src/tool.rs}` and tests
- `docs/adr/{0001,0004,0035,0072}*.md`, `crates/bingo/src/env.rs`,
  `crates/bingo-provider-openai/src/instances.rs`

## Exit criteria

- [x] Schedule's claim tests pass unchanged against `bingo-claim`.
- [x] A killed channel host leaves a claim the next start takes; a
      second live host is refused (pasted test output, before and after).
- [x] Doctor never offers to remove a modern channel claim.
- [x] `Reviewer` is refused when `reviewer` exists; `Parent` is refused.
- [x] An image-capable ACP agent receives the picture; another receives
      the note.
- [x] A 3 MB PNG from a plugin tool reaches the model ≤ 1 MB.
- [ ] `scripts/budget.sh` passes with the new crate; all gates (Windows blocked; see Risks).

## Non-goals

- The gateway pid file, the store's sidecar lock, the file shelf (M129).
- Merging agents and rooms (M122). Any ACP change beyond the prompt.

## Risks

- Existing `channels/*.lock` files on users' machines are legacy after
  this lands: the first start refuses with the remedy. Say so in the
  commit body and the channels guide.
- **Stopped, protocol step 6:** Windows plugin-rpc checking fails in transitive `aws-lc-sys`: `fatal error: 'windows.h' file not found`. Fixing the toolchain or the pictures dependency boundary is outside Files; no workaround or other-worktree edit was attempted.
  Options: provide a Windows SDK/C toolchain and rerun the remaining checks, or authorize a separate change to the picture library dependency boundary. M109 is not fully verified; ADR-0072 remains proposed.
- Re-verified at `82e78d22`: all named bug sites remain; workspace members
  already use `crates/*`, both name modules are private, and the guide is
  `crates/bingo-channels/src/guide.md`. No Decision changes are needed.

## Verified (2026-10-13)

Contract commits preceded implementation; trimmed output (before → after):

| Command | Before | After |
| --- | --- | --- |
| `cargo test -p bingo-claim` | `test result: FAILED. 0 passed; 8 failed;` | `test result: ok. 8 passed; 0 failed;` |
| `cargo test -p bingo-claim -p bingo-schedule --lib` | — | `test result: ok. 116 passed; 0 failed;` (schedule; original claim bodies moved unchanged) |
| `cargo test -p bingo --test channel_claim` | `successor exited instead of taking the claim: Ok(ExitStatus(unix_wait_status(256)))` | `test a_live_channel_host_excludes_another_and_a_killed_one_releases ... ok` |
| `cargo test -p bingo-gateway --lib modern_channel` | `test result: FAILED. 0 passed; 1 failed;` | `test doctor::tests::modern_channel_claims_are_probed_and_never_removed ... ok` |
| `cargo test -p bingo-agents --lib case_variants` | `test result: FAILED. 0 passed; 2 failed;` | `test names::tests::case_variants_cannot_claim_an_existing_or_reserved_name ... ok` |
| `cargo test -p bingo --test name_rules` | — | `test agents_and_rooms_compare_one_table_of_names_the_same_way ... ok` |
| `cargo test -p bingo-provider-acp --test pictures` | `test result: FAILED. 0 passed; 2 failed;` | `test result: ok. 2 passed; 0 failed;` (test adjusted to retain the existing opening preamble) |
| `cargo test -p bingo-plugin-rpc --lib plugin_picture` | `assertion failed: seen.decoded_len() <= bingo_pictures::MODEL_BUDGET` | `test tool::tests::a_three_megabyte_plugin_picture_is_bounded_before_the_model_sees_it ... ok` |
| `cargo test -p bingo-provider-openai --lib a_name_that_is_taken` | `test result: FAILED. 0 passed; 1 failed;` | `test instances::tests::a_name_that_is_taken_is_refused_by_name ... ok` |
| `cargo test -p bingo --test cli a_colliding_instance_name` | — | `test instances::a_colliding_instance_name_stops_the_boot_and_is_named ... ok` |

Gates run sequentially; logs are in `target/m109-logs/`:
- Initial workspace failures: the gateway test expected deleted channel files, then raced channel startup; both assertions were corrected. ACP interrupt timed out under parallel tests, then passed alone (`test result: ok. 1 passed; 0 failed;`); the final suite runs serially.
- `cargo fmt --all -- --check`: exit 0, no output.
- `cargo check --workspace --all-targets --locked`: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.50s``
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.97s``
- `cargo test --workspace --locked -- --test-threads=1`: exit 0; summed output: 4,960 passed, 0 failed, 3 existing ignored (stress/manual storyboard tests); final group: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.
- `cargo test -p bingo-provider-acp --test bridge --locked token -- --test-threads=1`: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.01s` (also included in the final serial workspace run).
- `scripts/check_discipline.sh`: `discipline ok` (historical size warnings).
- `scripts/budget.sh`: `dependencies (unique, normal): 343 (max  343)`; `warm cargo check -p bingo-core: 0s (max  20s)`; `budget ok`.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok` (duplicate-version warnings).
- `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`: claim/schedule/agents/rooms/provider-acp exit 0; plugin-rpc exit 101: `fatal error: 'windows.h' file not found`. Stopped immediately; channels/provider-openai/gateway/bingo Windows checks remain unrun.
