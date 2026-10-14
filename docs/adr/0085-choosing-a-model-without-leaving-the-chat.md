# ADR-0085 — Choosing a model without leaving the chat

Status: accepted · 2026-10-14 · Plans: M164 (§§1–4), M165 (§§5–7), M166 (§8) · Amends: ADR-0003 §5, ADR-0004 §3, ADR-0017 §6, ADR-0026 §4

## Context

The person switches models dozens of times a day and configures a
provider once a month; today both are equally hard and both leave the
chat. `/model ` completes hundreds of bare `provider/model` ids (dated
snapshots, embeddings and speech models among them) with no window,
reasoning, images or price beside them. `/model` always writes the user
settings layer (`commands/mod.rs:39` `remember`), so "try it in this
session" cannot be said. A named instance (ADR-0017 §6) is added by
`bingo provider add` before a kernel exists, then a restart. An
unreachable endpoint is found by the first message failing. ADR-0017 §4
says the auth status names where a key came from; `AuthStatus` has no
such field (`bingo-sdk/src/provider.rs:13`).

The person approved the design on 2026-10-14 in the interactive
prototype (alt+m / alt+t without a panel; `/model <filter>` as a list
and a fact card; providers managed, signed in and added in place).

The ratchet, per door: §2's catalogue keys, §4's `pinned`, §5's
`ProviderShape` and §6's `reach` and `from` each carry a fact the
kernel or a provider already owns; refusing them makes every surface
(terminal, desktop, chat) spell it again — a price table, a
dated-snapshot rule, an effort list per family, a second settings
parser for instances, a private probe. §7's verb exists because the
alternative is a surface writing a plugin's settings key.

## Decision

1. **A choice is for this session unless the person keeps it.**
   `/model <id>` and `/think <level>` reconfigure the session and write
   nothing. `--keep` (the picker's `tab`) also writes `provider`,
   `model` and `thinking` into the user layer through the one settings
   writer. `--model`/`--provider` stay never written. *(Amends ADR-0003
   §5's "a person who picks a model has picked one": the person's word
   2026-10-14 is that trying is the common case.)*
2. **`catalog(Models)` carries what a choice needs.** Beside `context`,
   `output`, `reasoning`, `images`: `price { input, output, cacheRead }`
   per million tokens, `chat` (the snapshot's `modalities.output` holds
   `text` and the id is not an embedding), `aliasOf` (a dated id whose
   undated prefix the same provider lists, by ADR-0004 §3's prefix
   rule), `efforts` (§3) and `pinned` (§4). Unknown to the snapshot:
   `chat: true`, the other facts absent. `scripts/models_dev.sh` keeps
   `cost` and `modalities.output`. *(Amends ADR-0004 §3, ADR-0026 §4.)*
3. **A provider says which efforts a model takes.**
   `Provider::efforts(model) -> Vec<Effort>`, default: every level when
   the model reasons, none otherwise. Anthropic answers its four,
   OpenAI its own; a level a provider folds into another (`Minimal` →
   `low`) is not listed.
4. **Pins are a kernel key.** `pinned = ["provider/model", …]`, order
   kept, written by `/model pin <id>` and `/model unpin <id>` into the
   user layer. alt+m steps the session through them; it is
   `/model <next>` without `--keep`.
5. **A wire shape is a contribution.** `Contribution::ProviderShape`
   (`openai`, `anthropic`, `codex`) builds one provider from a name and
   an instance spec (`baseUrl`, `images`) and names its settings key.
   The plugin builds its boot instances through the same shape, so an
   instance has one constructor.
6. **Reach and credential source are provider facts in
   `catalog(Providers)`.** Meta gains `shape`, `address`, `instance`
   (added by a person, removable), `reach` (`unchecked | ok { ms, at } |
   down { error, at }`, from the last list call, in memory) and
   `AuthStatus::Ready { from: Option<store | setting | env> }`. A list
   call (`/models refresh [<id>]`) updates `reach` and publishes
   `CatalogChanged { Providers }`. The picker asks for unchecked
   providers when it opens; ADR-0026 §4 still holds: never in a turn,
   never in `Host::build`, never for facts.
7. **`/provider` adds, changes and removes an instance at runtime.**
   `/provider add <name> <shape> <url>` checks the name (ADR-0017 §2),
   builds through §5, lists its models (§6), writes `<key>.instances.
   <name>` to the user layer and joins the live provider set; a shape
   that takes a key then opens the existing `Login { Paste }`
   interaction, so a key never enters command text or the journal.
   `/provider set <name> url <url>` and `/provider remove <name>` are
   its other verbs. `bingo provider add` calls the same code before a
   kernel exists. *(Amends ADR-0017 §6: registration is no longer
   boot-only.)*
8. **A switch the window cannot hold compacts first.** When the
   session's context is past the new model's compaction trigger, `/model`
   queues behind the turn, compacts on the current model, then
   switches; the outcome line says both sizes. A switch to a model
   without image input over a transcript with pictures says so in the
   same line. The picker is a site (ADR-0084).

## Consequences

- The picker, `/models`, `ListModels` and the desktop client read one
  catalogue; no surface holds a price, an effort list or a probe.
- Snapshot size grows by the cost and output-modality fields (budget
  run in M164). `catalog.rs`'s pinned meta-key test grows.
- `AuthStatus` and catalogue meta change on the wire: additive fields,
  fixtures updated, schema regenerated.
- A live provider set changes after boot: `ProviderSet::gather` already
  reads on every call; sessions keep the provider they resolved.
- A person who relied on `/model` persisting must say `--keep`; the
  outcome line says which was done.

Refs: ADR-0003, ADR-0004, ADR-0009, ADR-0012, ADR-0017, ADR-0026, ADR-0084.
