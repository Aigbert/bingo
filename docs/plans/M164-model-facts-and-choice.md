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
   pointing at ADR-0085; `docs/guide/` model page updated.

## Decisions

- The SDK cannot read kernel-owned reasoning metadata. `Provider::efforts`
  defaults to all levels; the catalogue applies the reasoning gate and omits
  efforts for unknown facts. This preserves ADR-0004's ownership boundary.
- Anthropic currently maps five distinct values, including `xhigh`, rather
  than the ADR's four. List the existing five; do not alter request encoding
  or cache behavior to fit a stale count.
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
- The workspace gate exposed an existing CLI helper that assumed the first
  multiplexed frame was the root. Select the print prompt's session instead;
  a resumed child's reminder may arrive first. Keep this test fix separate.
- `docs/guide/` did not exist; add `docs/guide/models.md` as the model page.

- Full upstream regeneration also changed existing request capabilities and
  dropped ids. Use `models_dev.sh --preserve-capabilities` to enrich this
  snapshot while retaining every old limit, reasoning flag and input modality.
  Missing output modalities on retained legacy ids stay chat candidates.

## Files

- `scripts/models_dev.sh`, `crates/bingo-core/models.dev.json`,
  `crates/bingo-core/src/models/{catalog.rs,mod.rs}`, new
  `models/alias.rs`.
- `crates/bingo-sdk/src/{provider,command}.rs`; `crates/bingo-provider-anthropic/
  src/request.rs`, `crates/bingo-provider-openai/src/` request mapping.
- `crates/bingo-core/src/host/{catalog,model_switch,pins}.rs`, `settings.rs`,
  `settings/merge.rs`, `commands/{mod,model,think,pin}.rs`,
  `session/inputs.rs`, `turn.rs`, `turn/compact.rs`, and their tests.
- `schema/`, `sdk/typescript/`, RPC fixtures.

## Exit criteria

- [ ] Snapshot parse, alias table, efforts per provider, meta key-set
      tests pass.
- [ ] `/model x` leaves the user settings file byte-identical;
      `/model x --keep` writes exactly `provider` and `model`
      (black-box, temp HOME).
- [ ] Pin / unpin round-trip in a TOML layer with comments preserved.
- [ ] Compact-before-switch and pictures line tested with the fake
      provider; the instant path stays instant.
- [ ] `scripts/budget.sh` run after the snapshot grows; size recorded.
- [ ] Gates: fmt, check, clippy -D warnings, test, check_discipline,
      budget, cargo deny, Windows check of the touched crates.

## Non-goals

The picker UI (M166); runtime providers (M165); live prices from an
endpoint; per-session pins; aliases across providers.

## Risks

- models.dev may lack `cost` for some ids: absent means unknown, never
  zero.
- The embedding rule must not hide a chat model; prefer showing a
  non-chat id over hiding a chat one, and test the snapshot's
  embedding ids by name.
- Compacting before a switch spends a model call on the current
  provider; if that provider is down, refuse the switch with the reason
  rather than switch into an overflow.
