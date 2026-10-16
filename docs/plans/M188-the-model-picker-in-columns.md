# M188 — The model picker in columns

ADR-0085, ADR-0095. The model picker in the prototype's layout and states.

## Goal

model-1 … model-8 match: header `13 models · 5 providers`, aligned columns,
card fields, highlighted dial, `◌ checking` / `✗ refused`, kind tags.

## Bricks

1. **Facts**: `Reach::Checking` and a typed reach error; `TextStyle` takes theme
   tokens and a background (for the dial and the pick row); fixture tests.
2. Header, aligned columns `★ ● name 200k ∴ ▣ provider`, `· N hidden`, card
   description and provider; `own place ▾` title goes.
3. Matcher: a model id matches before a provider prefix; `opus` gives the opus
   models, not every sonnet.
4. alt+m skips unsigned pins with a one-line toast; alt+t on a model with no
   dial toasts once; kind tags `older`, `snapshot`, `embeddings`, `images`,
   `voice` with `⏎ can't chat`.
5. Drive add-provider and codex sign-in against a local stub.

## Files

- `crates/bingo-mod-models/*`, `crates/bingo-sdk` (style, reach), `crates/bingo-surface-tui/src/pane/moved.rs`.

## Exit criteria

- [ ] Snapshot per model scenario; matcher table test.
- [ ] tmux drive with captures; gates as in M182.

## Non-goals

New providers; pricing data.

## Risks

- `Reach::Checking` must not delay a send: it is display-only.
