# ADR-0098 — A delivery may carry its headline

Status: accepted · 2026-10-19 · Plan: M196

## Context

A background job's completion has words for the model and a headline for the person.
Encoding its outcome in display glyphs inside those words makes a surface parse prose
and repeats the plugin's knowledge of the outcome. Refusing this door forces that second
representation: the delivery already knows its tone, but the persisted item loses it.

## Decision

- `bingo_sdk::headline::Headline { tone: Tone, parts: Vec<String> }` is re-exported by
  the sdk. `Tone` is the existing view vocabulary; each part is one line of inline Markdown.
- `Input::Text.headline` is optional, defaults to absent and is omitted when absent.
- The kernel retains it in the user item's `meta`, through typed accessors and one key,
  like `external`. One helper builds the user item for logging, starting and steering.
- Providers receive the user's content parts only. The headline never enters their context.
- A surface draws a headlined item as its headline in place of its text. An item without
  a headline keeps its existing rendering. `UserMessageProps` exposes the same headline
  to a mod that redraws the item.

## Rejected alternatives

- A `Slot::Row` placement: the delivering plugin does not know the item's id, and the
  placement does not persist with the transcript.
- Parsing the text: this is the string contract being removed; arbitrary text that begins
  with a checkmark must not acquire a different meaning.
- A new `ItemBody` variant: the notice remains the model's user message. A new variant
  would force every reader of user items to learn the same fact again.

## Consequences

Old inputs retain their byte-identical encoding. Headlines survive journal replay and
reach all surfaces, while model-facing background completion text stays unchanged.
The surface owns glyphs, ink, separators and motion; the sender owns parts and tone.
Contract fixtures pin both encodings and the item metadata accessor round trip.

## Supersedes

M196 round one's background-notice text parsing. Refs: ADR-0002, ADR-0008, ADR-0097.
