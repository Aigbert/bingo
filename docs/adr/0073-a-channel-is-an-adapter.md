# ADR-0073 — A channel is an adapter

Status: proposed · 2026-10-13 · Plan: M130 · Amends: ADR-0016 §1, §7

## Context

ADR-0016 §1 made `ChannelAdapter` crate-internal, and §7 made an
in-crate loopback the contract. That held for one platform. Today
`bingo-channels` is 14 482 lines, 6 619 of them Feishu. The trait exists
(`adapter.rs`), and Feishu reaches the rest of the crate through five
modules (`adapter`, `conversation`, `error`, `limits`, `question`) and one
constant. Yet the list of platforms is a closed list spelled out in
about six places (audit-2026-10 §5):

- `settings.rs`: `Channels { loopback, feishu, coalesce }`, `adapters`,
  `from_flags`;
- `secret.rs`: `adapters()`, a fixed list;
- the bin: `channels add` asks for "App id";
- the gateway's doctor: preflight prints "`bingo channels add feishu`".

A second platform means editing all of them, and every build of the
surface carries `reqwest` and `tokio-tungstenite` for Feishu alone.

The loopback is the local implementation the house rule asks for: no
real service needed, scripted by a test. But a packaged binary offers it
to anyone: `--channels loopback=<addr>` dials any address. The fake
provider shows the other way. The bin registers it only when
`BINGO_FAKE_SCRIPT` is set (`main.rs`, `plugins`).

No kernel door is opened. The kernel still knows no channel.

## Decision

1. **The contract is a library, `bingo-channel-adapter`**
   (`tier = "library"`). It holds what an adapter and the surface both
   speak:
   - `ChannelAdapter` and its mechanism traits, `Limits`, `Conversation`,
     `Question` and its ladder, and `ChannelError`;
   - `secret`, which looks credentials up through `bingo-auth-oauth`;
   - the test fixtures, behind a `testing` feature.

   The trait leaves crate-internal but does not enter the sdk.
2. **A platform is a value the binary hands over.** `trait Platform`
   provides:
   - `id`;
   - `parse` of its own settings slice, refusing in its own words;
   - `build` of an adapter from that slice and the environment;
   - its `secret` requirement;
   - `flag`, which says what `--channels <id>=<value>` means;
   - the prompts `bingo channels add <id>` asks.

   `ChannelsPlugin::new(platforms)` replaces the unit struct. The bin
   builds the list once and hands the same list to the gateway's doctor
   and to `channels add`. No crate but the bin names a platform.
3. **Settings keep their shape.** `channels.<id>` is the platform's
   slice. The surface reads `coalesce` and each slice's `access`, and
   hands the platform the rest. The surface's schema leaves platform
   slices open, because no one validates a schema at runtime today.
   `channels.feishu.appId` means what it meant, and so does
   `--channels feishu`. A slice no platform claims is refused, and the
   refusal names it.
4. **Each platform is its own crate.** Feishu moves to
   `bingo-channel-feishu`, with its HTTP, websocket, cards and fixtures.
   It is plugin tier, so `check_discipline.sh` refuses
   `bingo-channels → bingo-channel-feishu` as it refuses any plugin
   edge. Only the bin depends on it.
5. **The loopback ships off.** It moves into `bingo-channel-adapter` as
   the contract's own platform, and §7's role stands: every Deliverer
   behaviour is proven on it first. The bin adds it to the list only
   when `bingo_channel_adapter::loopback::ENABLE_ENV`
   (`BINGO_CHANNEL_LOOPBACK`) is set, the same gate the fake provider
   uses. Without it, `--channels loopback=…` is refused, and the refusal
   names the variable. The black-box tests set it.

## Consequences

- A new platform is a new crate and one line in the bin.
  `bingo-channels` stops depending on `reqwest`, `tokio-tungstenite`
  and `base64`. Two members are added, so `max_dependencies` rises by
  two.
- `ChannelAdapter` becomes an API between workspace crates. Changing
  it touches every platform crate, which is the price of having two.
- A packaged binary no longer dials a loopback peer. This is a
  behaviour change, named in the channels guide and the commit body.
- `scripts/feishu-smoke.md` keeps every command. Only the crate name in
  `cargo test -p` changes.
- §5's one-process-per-app claim is unchanged in meaning; its mechanism
  is `bingo-claim` (ADR-0072, M109).

Refs: ADR-0001, ADR-0012, ADR-0016, ADR-0042, ADR-0072; Plan: M130;
audit-2026-10 §5
