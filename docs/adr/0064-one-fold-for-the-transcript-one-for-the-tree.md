# ADR-0064 — One fold for the transcript, one for the tree

Status: proposed · 2026-10-13 · Plans: M112 (transcript), M120 (tree) · Amends: ADR-0002 §3

## Context

ADR-0002 §3 named two reducers over one journal: `SessionState::apply`
for clients and `ContextView::fold` for the model. In the tree there are
three folds of the transcript, each with its own upsert, compaction
splice and rewind:

- `SessionState::apply` (`bingo-sdk/src/state.rs`), which also honours
  `TurnRetrying { dropped }`;
- `ContextView::items` (`bingo-core/src/context.rs`), an O(n²)
  `iter_mut().find` per item frame, which does not;
- the turn's own `self.items` (`bingo-core/src/turn.rs`), which upserts,
  splices at compaction and drops a retry's items again by hand.

They have already diverged: text withdrawn by a retry is gone from the
client and the running turn, then comes back in the next turn's request
and after resume (M108 fixes the symptom). A fourth copy of the work is
the clone of the whole journal into every turn
(`session/running.rs`, `history: actor.journal.clone()`), which the
model view then re-folds from the first frame.

The tree has the same shape one level up. Which session is a child of
which, and when one goes away, is folded three times from the
gateway's `(SessionId, Frame)` stream: the TUI's `tree.rs`, the
channels runner's `state_of` and `--print`'s `state_of`. Only the TUI
removes a child; the other two keep a deleted one forever.

No kernel door is opened: this moves an existing fact into one type.

## Decision

1. **`bingo_sdk::transcript::Transcript` is the one fold of items.** It
   owns the item list, an `ItemId → index` map, the history generation,
   the model order (the ids the model sees, in order, kept beside the
   client list over the same `Arc<Item>`s), and the rules: upsert by id;
   a new item joins both orders; `TurnRetrying.dropped` and
   `Rewound.dropped` remove ids from both orders. Each `Compacted` cut
   is applied to the model order once, as it is folded, by the splice
   `ContextView` uses today; an unknown boundary makes no cut.
   The model order is not derivable from the client list plus any one
   cut (two cuts yield `[s2, b]`, not `[s2, b, s1]`), so it is its own
   fact, not a copy. Items are `Arc<Item>`, so a snapshot shares bodies.
2. **Two views of one fold.** `SessionState` embeds the `Transcript` and
   stops keeping `items` and `history_generation` of its own; a client
   sees every item, as today. `ContextView` becomes the *model view*: it
   renders the model order, skips external tool calls
   (`Item::external()`), and renders `Message`s. It no longer reads
   frames.
3. **The turn folds through it.** The turn is handed a `Transcript`
   snapshot, not frames, and applies each event it emits to its own copy
   through the same `apply`; `self.items` and its hand-written splice
   and retain are deleted. The actor clones no journal to start a turn.
4. **A property test is the contract:** for any frame sequence, folding
   incrementally equals folding from scratch, `SessionState`'s items
   equal the transcript's, and the model view never contains an id that
   a `TurnRetrying` or `Rewound` dropped. For any frame sequence, the
   transcript's model view equals today's frame-based
   `ContextView::items`; keep that function under `cfg(test)` as the
   oracle through M112, with a named two-compaction regression.
5. **`bingo_sdk::tree::TreeState` is the one fold of the tree.** It folds
   `(SessionId, Frame)` and `GatewayEvent`s for one root and its
   descendants. One creation rule: a child appears on its
   `SessionUpdated`; any other frame for an unknown session is stale
   and ignored. One removal rule: `SessionClosed { Deleted }` and
   `GatewayEvent::SessionRemoved` take the session and its subtree out.
   It holds a `SessionState` per session and the parent links, nothing
   else.
6. **Surface state stays the surface's.** Which child the TUI is
   showing, its connection handles, a chat's message map are not tree
   facts and stay where they are. The words a surface uses for a child's
   lifecycle are ADR-0077's.

## Consequences

- One place to change when a fifth event touches items; the retry bug
  class is unrepresentable. `ContextView::fold(frames)` survives only as
  a test helper that folds a `Transcript` first.
- A turn starts in constant time in the journal's length; the request
  is still assembled from all live items (cutting that is compaction's
  job, ADR-0006).
- `SessionState` serialises as before; `Transcript`'s map is skipped and
  rebuilt on deserialise, so the wire does not change.
- Channels and `--print` gain removal of deleted children; the TUI
  loses its own fold and keeps its view selection.
- The journal is still the record and the source of replay; working
  memory beyond the transcript is ADR-0066's.

Refs: ADR-0002, ADR-0006, ADR-0045, ADR-0077; Plans: M108, M112, M120;
audit-2026-10 §1, §4
