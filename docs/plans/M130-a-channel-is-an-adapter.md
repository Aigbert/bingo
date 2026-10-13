# M130 — A channel is an adapter

## Goal

ADR-0073. `bingo-channels` keeps the deliverer, the runner, access, the
directory and `SendFile`. The contract moves to a library,
`bingo-channel-adapter`, and Feishu (6 619 of 14 482 lines) to
`bingo-channel-feishu`. The bin builds the list of platforms once. The
loopback ships off behind a switch. A Feishu user sees no change in a
setting, a flag, the wire or a card.

## Bricks, in build order

1. **The library, by moving.** Create `crates/bingo-channel-adapter`,
   `tier = "library"`, on `bingo-sdk`, `bingo-auth-oauth` and what the
   moved files already use. Move `adapter.rs`, `conversation.rs`,
   `error.rs`, `limits.rs`, `question.rs` and `secret.rs` with their
   tests, and `fixtures.rs` behind a `testing` feature. Keep the fixed-list
   secret queries in the surface until brick 4 removes that dependency. The `pub use`
   lines in `bingo-channels` keep their names and point at the library,
   so no caller changes yet. The channels test count holds (paste).
2. **The platform contract, tests first.** `trait Platform` in the
   library: `id`, `build(slice, env)` (which parses its own slice and
   refuses in its own words), `secret() -> Option<Requirement>`,
   `flag(value) -> Value` (the slice `--channels <id>=<value>` stands
   for) and the prompts `channels add` asks. A test platform lives under
   `testing`. These tests fail first (paste):
   - `channels.telegram`, with no platform of that id, is refused, and
     the refusal names `telegram`. Today it is silently ignored.
   - A slice its platform refuses is refused in the platform's words.
   - An unknown `--channels` id keeps today's "no such channel adapter".
3. **The loopback becomes the contract's platform.** `loopback.rs`,
   with `base64` and tokio's `net` feature, moves to
   `bingo_channel_adapter::loopback` and implements `Platform` (its flag
   is `{"peer": value}`). `ENABLE_ENV` (`"BINGO_CHANNEL_LOOPBACK"`) and
   `enabled(env)` sit beside it, because the bin never spells a
   `BINGO_` name (`bin/env.rs`). It is not the `bingo-loopback` socket
   crate. Every Deliverer test still runs on it (ADR-0016 §7).
4. **The surface takes a list.** `ChannelsPlugin::new(platforms)`
   replaces the unit struct. In `settings.rs`, `Channels` becomes
   `coalesce` plus the slices by id. `adapters`, `access`, `gate`,
   `wanted` and `from_flags` take the list. `secret::configured` and
   `signing` ask each platform, not a fixed list. An id missing from the
   list is refused; for `loopback` the refusal adds the name of
   `ENABLE_ENV`. `schema()` describes `coalesce` and leaves the slices
   open. The settings tests pass, changed only to hand over a list.
5. **Feishu moves out.** `src/feishu/**` and its snapshots become
   `crates/bingo-channel-feishu`, plugin tier, with `reqwest`,
   `tokio-tungstenite`, `base64` and `bingo-pictures`. `FeishuChannel`,
   `APP_SECRET` and its `Platform` move too. Its flag is
   `{"appId": value}`, and it prompts for the app id, then the secret.
   Its tests take `fixtures` from the library's `testing` feature.
   `bingo-channels` drops those four dependencies. Paste the count of
   `feishu::` tests under `-p bingo-channels` before the move and under
   `-p bingo-channel-feishu` after it. They match.
6. **The bin builds the list once.** In `main.rs`, `platforms(env)`
   returns Feishu always, and the loopback only when
   `loopback::enabled`. `plugins()` hands it to `ChannelsPlugin::new`,
   and the callers of `from_flags` and `wanted` hand it over too.
   `channels.rs` `add`, `secret` and `signing` take their prompts and
   requirement from the platform. The gateway's `preflight`,
   `credential_checks` and `merged_channels` take the list from the bin.
   The "no channel is configured" line names `bingo channels add <id>`
   for each platform that signs. The doctor's tests keep their Feishu
   cases through a dev-dependency, so its words stay pinned.
7. **Black-box.** `crates/bingo/tests/channels.rs` sets the switch. A
   new test runs `--channels loopback=127.0.0.1:9` without it: non-zero
   exit, empty stdout, and the variable named on stderr.
8. **Records.** Update `crates/bingo-channels/src/guide.md` (the
   loopback is off unless switched on). In `scripts/feishu-smoke.md`,
   `cargo test -p bingo-channels` becomes `-p bingo-channel-feishu`.
   ADR-0016 gets a dated note pointing at ADR-0073.

## Files

- `crates/bingo-channel-{adapter,feishu}/**` (new), `Cargo.toml`,
  `scripts/budget.toml`, `Cargo.lock`
- `crates/bingo-channels/{Cargo.toml,src/**}`
- `crates/bingo/{Cargo.toml,src/{main,channels}.rs}`
- `crates/bingo/tests/{channels.rs,cli/gateway.rs}`
- `crates/bingo-gateway/{Cargo.toml,src/{doctor,lib}.rs}`
- `scripts/feishu-smoke.md`, `docs/adr/0016-*.md`

## Exit criteria

- [ ] Brick 2's tests and brick 7's refusal pass (paste before and after).
- [ ] `cargo tree -p bingo-channels -e normal --depth 1 | rg
      'reqwest|tungstenite|base64|bingo-pictures'` finds nothing.
- [ ] `rg -il feishu crates/bingo-channels crates/bingo-channel-adapter`
      finds nothing. In `crates/bingo-gateway/src`, only test modules
      match (paste).
- [ ] `cargo test -p bingo-channel-adapter -p bingo-channels
      -p bingo-channel-feishu -p bingo-gateway` passes, with brick 5's
      counts equal, and so does `cargo test -p bingo --test channels`.
- [ ] `scripts/check_discipline.sh` refuses a scratch
      `bingo-channels → bingo-channel-feishu` edge (paste, then revert).
- [ ] `scripts/budget.sh` (+2), `cargo deny check`, and the Windows
      `cargo check` of the three channel crates pass. All gates pass.

## Non-goals

- A second platform (Telegram, Slack).
- Any change to the Deliverer, the question ladder, Feishu's wire, or
  the sdk (`ChannelAdapter` stays out of it).
- The live Feishu smoke, owed since M83–M86. It does not gate this plan.
- The channels' claim (M109).

## Risks

- Re-verified at `eddd0ebc`: M109 claim and M112 transcript folds remain;
  M129 gateway dispatch still constructs `Patient` in `src/lib.rs`, so
  that path and both composition manifests are included. Gateway CLI
  loopback cases also need the switch. `Cargo.lock` records the two members.
  Schema claims remain descriptive, with no runtime schema validator.
  The shared channel-directory constant moves to the library with a
  re-export from `lock`; no claim behavior changes.

- Snapshots move with the files. Run with `INSTA_UPDATE=no`, so a
  moved one is never rewritten silently.
- Moving modules widens `pub(crate)` items to `pub`. Keep each one
  `#[doc(hidden)]` or behind `testing` where only tests reach it.
- Gating the loopback changes what a packaged binary accepts. Name it
  in the commit body and the guide.
- The open slices rest on no runtime schema validation (only provider
  tests consume schemas today). Re-verify it.
- M109 (the claim), M127 (Feishu's `api.rs` client) and M119/M126
  (composition) touch these files. Follow the tree.
- Re-verify at start: every line and symbol named here, and where the
  bin reads `--channels`.
