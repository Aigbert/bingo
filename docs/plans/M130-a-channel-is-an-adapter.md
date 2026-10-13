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
- `crates/bingo/tests/{channels.rs,channel_claim.rs,cli/gateway.rs,support/mod.rs}`
- `crates/bingo-gateway/{Cargo.toml,src/{doctor,lib}.rs,tests/claim.rs}`
- `scripts/feishu-smoke.md`, `docs/adr/0016-*.md`

## Exit criteria

- [x] Brick 2's tests and brick 7's refusal pass (paste before and after).
- [x] `cargo tree -p bingo-channels -e normal --depth 1 | rg
      'reqwest|tungstenite|base64|bingo-pictures'` finds nothing.
- [x] `rg -il feishu crates/bingo-channels crates/bingo-channel-adapter`
      finds nothing. In `crates/bingo-gateway/src`, only test modules
      match (paste).
- [x] `cargo test -p bingo-channel-adapter -p bingo-channels
      -p bingo-channel-feishu -p bingo-gateway` passes, with brick 5's
      counts equal, and so does `cargo test -p bingo --test channels`.
- [x] `scripts/check_discipline.sh` refuses a scratch
      `bingo-channels → bingo-channel-feishu` edge (paste, then revert).
- [x] `scripts/budget.sh` (+2), `cargo deny check`, and all required local gates pass.
- [ ] Windows builds: all five touched crates resolve `aws-lc-sys`; left to Windows CI as instructed.

## Non-goals

- A second platform; changes to the Deliverer, question ladder, wire, sdk or M109 claim.
- The live Feishu smoke, owed since M83–M86, does not gate this plan.

## Risks

- **Authorized (2026-10-13):** add the platform list to M129's gateway
  claim test without changing its assertions. For the rest of M130, tests
  or fixtures outside Files may be updated mechanically when they only
  construct or name a changed type; record those paths here. Changes to
  behavior or assertions outside Files still require a protocol stop.
  `tests/channel_claim.rs` and `tests/support/mod.rs` only gain the loopback switch.
- Re-verified at `eddd0ebc`: M109 claim, M129 gateway and M112 folds remain.
  Composition manifests, gateway dispatch and loopback CLI fixtures are included.
  The directory constant moves to the library; claim behavior stays unchanged.
- Schemas remain descriptive; platform slices have no runtime schema validator.
- Move snapshots unchanged; run with `INSTA_UPDATE=no`.
- Newly public internals stay `#[doc(hidden)]` or behind `testing`.
- Name the packaged loopback opt-in change in the commit body and guide.

## Verified (2026-10-13)

- Baseline `cargo test -p bingo-channels -- --list`: `251 tests`; `120` named `feishu::`.
- Brick 1 `INSTA_UPDATE=no cargo test -p bingo-channel-adapter -p bingo-channels`: `22 passed; 0 failed` + `229 passed; 0 failed` = 251; initial budget `346 (max 347)`, `budget ok`.
- Contract red `cargo test -p bingo-channel-adapter platform::tests`: `1 passed; 2 failed`;
  `Ok([])` instead of `Err("no such channel adapter: telegram")` / `Err("sample refuses this slice")`.
- Contract green is included in the adapter's `33 passed; 0 failed` below; unknown flag wording stays pinned.
- `INSTA_UPDATE=no cargo test -p bingo-channel-adapter -p bingo-channels -p bingo-channel-feishu -p bingo-gateway`:
  adapter `33 passed`, surface `102 passed`, platform `120 passed`, gateway `50 passed` + claim `8 passed`; all `0 failed`.
  Feishu count remains 120; snapshots unchanged. Initial missing `Patient.platforms` resolved by authorized fixture wiring.
- `INSTA_UPDATE=no cargo test -p bingo --test channels loopback_without_the_switch_is_refused_before_listening`:
  red `0 passed; 1 failed`, stderr `the loopback peer 127.0.0.1:9: Connection refused` (missing switch remedy).
  Green `INSTA_UPDATE=no cargo test -p bingo --test channels`: `16 passed; 0 failed`; stdout/status/remedy asserted.
- `cargo tree -p bingo-channels -e normal --depth 1 | rg 'reqwest|tungstenite|base64|bingo-pictures'`: no matches (exit 1).
- `rg -il feishu crates/bingo-channels crates/bingo-channel-adapter`: no matches (exit 1).
  Gateway matches only `doctor.rs` / `unit.rs` test modules; no production matches.
- Scratch `scripts/check_discipline.sh`: exit 1, `bingo-channels -> bingo-channel-feishu (plugin -> plugin; use a service trait via the sdk)`; manifest and lock restored.
- `cargo fmt --all -- --check`: exit 0, no output.
- `cargo check --workspace --all-targets --locked`: `Finished dev ... in 22.74s`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev ... in 10.59s`; two single-element guide loops fixed.
- `INSTA_UPDATE=no cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing`:
  exit 0; summed `5000 passed; 0 failed; 3 ignored` (existing stress/manual tests); no flaky reruns needed.
- `INSTA_UPDATE=no cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1`:
  `1 passed; 0 failed; 0 ignored; 2 filtered out`. No parallel Cargo jobs; workspace PTY suite also passed.
- `scripts/check_discipline.sh`: `discipline ok` (historical size warnings).
- `scripts/budget.sh`: `347 (max 347)`; core `0s`; isolation `0` recompiled; target `5 GB`; `budget ok`.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok` (duplicate-version warnings).
- Sequential `cargo tree -p <crate> -i aws-lc-sys --target x86_64-pc-windows-msvc --locked`:
  adapter, platform, channels, gateway and bin each return `aws-lc-sys v0.44.0`; Windows CI owns all five builds. No toolchain installed.
- [ ] Feishu live smoke: requires real credentials; not attempted as instructed. No TUI-byte changes or separate TUI smoke.
