# M164 — Model facts and choice

ADR-0085 §§1–4.

## Goal

Every surface can show, per model, its window, output, price, image
input, effort levels, whether it chats, whether it is a dated alias and
whether the person pinned it, all from `catalog(Models)`. `/model` and
`/think` change this session only unless `--keep` is given; pins are a
settings key.

## Bricks

1. **Snapshot fields.** `scripts/models_dev.sh` also keeps
   `cost.{input,output,cache_read}` and `modalities.output`; regenerate
   `crates/bingo-core/models.dev.json` with `--preserve-capabilities`. `ModelFacts` gains
   `price: Option<Price>` (`input`, `output`, `cache_read`, per million
   tokens) and `chat: bool` (output holds `text`, id not an embedding by
   the snapshot's family). Parse test on a three-model fixture.
2. **Alias rule, pure.** `models::alias_of(id, listed) -> Option<&str>`:
   a dated id (`-YYYYMMDD` or `-YYYY-MM-DD` suffix) whose undated prefix
   is in `listed`. Table test, including a dated id with no undated twin
   (not an alias).
3. **Provider efforts.** `Provider::efforts(&self, model) -> Vec<Effort>`
   in `bingo-sdk`, default all levels, catalogue-gated by `reasoning`. Providers answer
   from their own request mapping (`request.rs`); a level folded into
   another is not listed. Unit test per provider, plus one asserting
   every listed level maps to a distinct wire value.
4. **Catalogue meta.** `catalog.rs::model_meta` adds `price`, `chat`,
   `aliasOf`, `efforts`, `pinned`; the pinned key-set test grows; RPC
   fixture and `schema/` regenerated. Unknown ids: `chat: true`, the
   rest absent.
5. **Pins.** Kernel key `pinned` (`KERNEL_KEYS`, `KernelSettings`,
   merge test: `Replace`). `/model pin <id>` / `/model unpin <id>` write
   it through `settings::remember`, then publish
   `CatalogChanged { Models }`. A pin of an unknown id is refused with
   the closest ids.
6. **Scope.** `/model <id> [--keep]` and `/think <level> [--keep]`:
   `remember` only with `--keep`. Outcome lines:
   `⇄ <model> · <provider> · thinking <x> · this session` or
   `· kept for new sessions`. `/model` with no id unchanged. Argument
   spec gains the flag so completion offers it.
7. **Compact before switch.** `/model` reads the session's usage and
   the target's resolved window (`resolve_model`); past the target's
   compaction trigger it stops being instant: it queues, compacts with
   the current model (`Host::compact`), then reconfigures. Outcome adds
   `compacted <before> → <after>`. A target without image input over a
   transcript holding pictures adds `pictures stay, <model> won't see
   them`. Unit tests with the fake provider for both, and for the
   ordinary instant path.
8. **Docs.** ADR-0003 §5 and ADR-0004 §3 get a dated amendment line
   pointing at ADR-0085; the `guide-tui` page gains "Choosing a model".

## Decisions

- The SDK cannot read kernel-owned reasoning metadata. `Provider::efforts`
  defaults to all levels; the catalogue gates known reasoning facts (ADR-0004).
- Anthropic maps five distinct values, including `xhigh`, rather than the
  ADR's four. List those five without altering request encoding or caching.
- Invocation-specific `Command::is_instant` defaults to the static spec.
  Model choice resolves its target against the supplied session state before
  admission; the actor rechecks queued commands when dispatching them.
- `--keep` persists only the command's own keys (model/provider or thinking),
  as the exit criteria require; it never overwrites an unrelated choice.
- Pin order is represented by stable catalogue ordering (pins first). Unknown
  snapshot ids omit unavailable facts, but an explicitly pinned offered id
  still reports its known pin state. Stale pins can always be removed.
- Admission uses the larger of measured usage and the shared transcript
  estimate so a new unfinished input cannot hide behind an old measurement.
  Compactor counts cover only the replaced prefix, so manual compaction
  measures the retained transcript with the already assembled system/tools
  and publishes usage through the existing event before switching. It must
  not rerun contributors: snapshot refresh and cached baselines stay lazy.
- A compaction that cannot get below the target trigger refuses the switch;
  a same-model choice never compacts solely because of this command.
- The CLI helper must find the print prompt's session: a resumed child can
  emit the first multiplexed frame. Keep this gate-exposed test fix separate.
- `cargo deny` exposed RUSTSEC-2026-0346 through 0349 in rustls 0.23.45.
  Update only that existing lockfile package to 0.23.46; add no dependency.
- Guides are per-crate pages; the model section lives on `guide-tui` (crates/bingo-surface-tui/src/guide.md) until M166 gives `bingo-mod-models` its own page.
- Full upstream regeneration also changed existing request capabilities and
  dropped ids. Use `models_dev.sh --preserve-capabilities` to enrich this
  snapshot while retaining every old limit, reasoning flag and input modality.
  Missing output modalities on retained legacy ids stay chat candidates.

## Files

- `scripts/models_dev.sh`, `crates/bingo-core/models.dev.json`,
  `crates/bingo-core/src/models.rs`, `models/{catalog,resolve,alias}.rs`.
- `crates/bingo-sdk/src/{provider,command}.rs`; `crates/bingo-provider-anthropic/
  src/request.rs`, `crates/bingo-provider-openai/src/` request mapping.
- `crates/bingo-core/src/host/{catalog,model_switch,pins}.rs`, `settings.rs`,
  `settings/merge.rs`, `commands/{mod,model,think,pin}.rs`,
  `session/inputs.rs`, `turn.rs`, `turn/compact.rs`, and their tests.
- `schema/`, `sdk/typescript/`, RPC fixtures.

## Exit criteria

- [x] Snapshot parse, alias table, efforts per provider, meta key-set
      tests pass.
- [x] `/model x` leaves the user settings file byte-identical;
      `/model x --keep` writes exactly `provider` and `model`
      (black-box, temp HOME).
- [x] Pin / unpin round-trip in a TOML layer with comments preserved.
- [x] Compact-before-switch and pictures line tested with the fake
      provider; the instant path stays instant.
- [x] `scripts/budget.sh` run after the snapshot grows; size recorded.
- [x] Native gates: fmt, check, clippy -D warnings, test, discipline, budget, deny.
- [ ] TUI smoke: both runs stop at counter input `Name: Ad`, expected `Name: Ada`.
- [ ] Windows: three crates pass; four have permitted C-toolchain deferrals below.

## Non-goals

The picker UI (M166); runtime providers (M165); live prices from an
endpoint; per-session pins; aliases across providers.

## Risks

- models.dev may lack `cost`: absent means unknown, never zero.
- Prefer showing a non-chat id over hiding a chat model; test the snapshot's
  embedding ids by name.
- Compacting before a switch spends a model call on the current
  provider; if that provider is down, refuse the switch with the reason
  rather than switch into an overflow.

## Verified — 2026-10-15

Sequential gates; offline model fixtures. Snapshot: 12 providers, 287 models, 119135 bytes (+50937); every previous limit/reasoning/input capability retained.

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --locked => 185 passed
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods --locked => 332 passed, 2 ignored
cargo fmt --all -- --check => exit 0
cargo check --workspace --all-targets --locked => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
cargo test --workspace --locked => 5509 passed; 5 ignored; 163 groups
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
bash scripts/budget.sh => dependencies 411; warm core 0s; isolation 0; budget ok
cargo deny check => advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh => exit 1 twice: timed out waiting for: Name: Ada
cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc:
  bingo-sdk, bingo-core, bingo-surface-rpc => exit 0
  bingo-provider-anthropic, bingo-provider-openai, bingo-surface-tui => exit 101: aws-lc-sys
  bingo => exit 101: rquickjs-sys and aws-lc-sys; Windows C headers/toolchain absent
```
Budget's 12 GB build-directory soft warning is addressed by final `cargo clean`.
The smoke's counter input failure reproduced in fresh isolated sockets/homes;
that path is unchanged by M164. The milestone is not fully gate-verified.
