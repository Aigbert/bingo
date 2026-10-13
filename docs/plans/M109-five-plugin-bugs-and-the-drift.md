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
     files on disk do not change.
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

- `crates/bingo-claim/**` (new), `Cargo.toml` (workspace member)
- `crates/bingo-schedule/src/{lock.rs,lock/*}` → re-export or callers
- `crates/bingo-channels/src/lock.rs`, `crates/bingo-channels/src/host.rs`
- `crates/bingo-gateway/src/doctor.rs` and tests
- `crates/bingo-agents/src/names.rs` and tests; `crates/bingo/tests/`
- `crates/bingo-provider-acp/src/provider.rs` and tests
- `crates/bingo-plugin-rpc/{Cargo.toml,src/tool.rs}` and tests
- `docs/adr/{0001,0004,0035,0072}*.md`, `crates/bingo/src/env.rs`,
  `crates/bingo-provider-openai/src/instances.rs`

## Exit criteria

- [ ] Schedule's claim tests pass unchanged against `bingo-claim`.
- [ ] A killed channel host leaves a claim the next start takes; a
      second live host is refused (pasted test output, before and after).
- [ ] Doctor never offers to remove a modern channel claim.
- [ ] `Reviewer` is refused when `reviewer` exists; `Parent` is refused.
- [ ] An image-capable ACP agent receives the picture; another receives
      the note.
- [ ] A 3 MB PNG from a plugin tool reaches the model ≤ 1 MB.
- [ ] `scripts/budget.sh` passes with the new crate; all gates.

## Non-goals

- The gateway pid file, the store's sidecar lock, the file shelf (M129).
- Merging agents and rooms (M122). Any ACP change beyond the prompt.

## Risks

- Existing `channels/*.lock` files on users' machines are legacy after
  this lands: the first start refuses with the remedy. Say so in the
  commit body and the channels guide.
- `bingo-claim` must build on Windows (`File::try_lock` is std);
  cross-check it and the channels crate.
- Re-verify at start: names and paths above, by symbol.
