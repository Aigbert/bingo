# M201 — Why it ran, and how long it took

ADR-0002 (one stream). Every field added here is optional and skipped when absent. Spec:
`docs/design/ember/` (`p4b-data.js` `answeredBy`, `QF_READ`; `p4-model.js` `pauseFor`,
`elapsedOf`; `p5-stream.js` `clockSegs`, `whySegs`, `resultSegs`).
After: M192. Runs beside M193 and M194 part B, and touches no surface.

## Goal

Three facts the Ember rows draw are thrown away today, so no surface can draw them honestly:

- **Why a call ran without asking.** The policy decides `Allow { reason }`, and core drops the
  reason (`gate.rs` `resolve_check`). A step that ran because of a mode or a rule the person
  added this session says so on its row (`accept edits`, `bypass`, `session rule`).
- **How long a turn waited for you.** An interaction's `opened_at` is dropped when it closes, so
  a turn's clock cannot leave out the time spent waiting. Time spent waiting for you is not
  bingo's time (`clockSegs`).
- **How much Read read.** A text read has no upshot, so its row cannot say `214 lines`.

This milestone records them where they happen and gives every surface one rule to read them.
Drawing them is M194 part B's.

## Bricks

1. **A call remembers why it ran without asking** (contract first).
   - `ItemBody::ToolCall` gains one optional field, skipped when absent, that holds the policy's
     reason when the policy allowed a call that its default would have asked about:
     - a mode: `acceptEdits` or `bypassPermissions`;
     - an allow rule, with whether it came from settings or was added in this session;
     - a hook that allowed it.
   - A read-only allow (`Reason::ReadOnly`) records nothing, since nothing would have asked. A
     call that was asked records nothing here either, because its receipt says what happened.
   - The policy keeps the rule's source. `bingo-permissions` `rules_for` merges session rules
     into the settings allow list today, so `Reason::Rule` cannot tell them apart. Keep the
     source through the match, and add it to `Reason::Rule` as an optional field.
   - Core writes the field once, in the `ItemUpdated` that marks the call `Running` at gate time
     (`turn.rs` `gate`).
2. **A receipt remembers when its ask opened** (contract first).
   - `ItemBody::PermissionReceipt` and `ItemBody::QuestionAnswer` gain an optional `asked_at`: the
     interaction's `opened_at`, filled by core when it records the receipt.
   - A journal written before this milestone reads as before, with no `asked_at`.
3. **One clock for a turn** (a pure SDK function, with unit tests before its callers).
   - Input: the session state, a turn id and now. Output: the turn's elapsed time without its
     waits, and the instant the next whole second is due while it ticks.
   - It starts at the turn's first item. It ends:
     - at now, for a live turn;
     - at the open ask's `opened_at`, while one is open;
     - at the last item's completion, for a finished turn.
   - It subtracts each of the turn's receipts' waits: the receipt's own time minus `asked_at`.
   - Every turn uses the same rule, the live one, the last one and older ones alike, so a band's
     clock never changes when the next turn starts.
4. **Read says how much it read.**
   - `bingo-tool-fs` Read sets an upshot.
     - A text file: `N lines`, where N is the file's line count, also when the read returned
       only part of it (`QF_READ.of`).
     - A picture: `W×H` when the decoder knows the size.
   - What the model reads is unchanged.
5. **Schemas and types.** Regenerate the schemas with `BINGO_UPDATE_SCHEMA=1`, and update the
   TypeScript types in `sdk/typescript`. `--print` and the RPC carry the new fields as they
   carry any optional field.

## Files

- `crates/bingo-sdk/src/{event.rs,policy.rs}`, and the clock in `transcript/` or `state.rs`
  beside what it reads, with the schema fixtures and `sdk/typescript`.
- `crates/bingo-core/src/{gate.rs,turn.rs}`, and wherever core records a receipt.
- `crates/bingo-permissions/src/{decide.rs,session.rs,lib.rs}`.
- `crates/bingo-tool-fs/src/{read.rs,upshot.rs}`.

## Exit criteria

- [ ] Schema or fixture tests for each new field land before the code that fills it. The
      regenerated schemas and TypeScript types are committed.
- [ ] A journal from before M201 replays unchanged. A journal with the fields round-trips.
- [ ] Core tests, one per path:
  - an edit inside the cwd in `acceptEdits` records the mode;
  - any call in `bypassPermissions` records the mode;
  - a call allowed by a rule added with "yes for this session" records the rule and its
    session source;
  - a settings allow rule records its settings source;
  - a hook's allow records the hook;
  - a read-only call and an asked call record nothing on the call.
- [ ] A permission ask and a question answered after a wait record `asked_at` equal to the
      interaction's `opened_at`.
- [ ] Clock unit tests:
  - no asks;
  - one closed ask, subtracted;
  - an open ask freezes the clock at its `opened_at`;
  - a stopped turn;
  - a receipt without `asked_at` subtracts nothing;
  - the newest turn and an older one read alike.
- [ ] Read tests: `N lines` for a whole read and for a partial read of the same file, and `W×H`
      for a picture.
- [ ] `--print` and RPC black-box tests pass. A test that pins whole JSON is re-pinned only for
      the new optional fields, and each such test is named in Verified.
- [ ] Gates, one at a time, and the Windows check for each crate touched.

## Non-goals

- Drawing anything. M194 part B draws the why and the waited-out clock after this merges.
- When the kernel asks. Today it gates every call of a round before running any, and that is
  M202's question.
- Removing or renaming a wire field. `LastTurn` and `LiveTurn` keep their clocks for their
  readers (agents, channels, mods, the RPC snapshot); the band reads only the clock function.

## Risks

- **The rule's source.** Session rules are merged into `rules.allow` before `decide`, so the
  source has to survive the merge. If keeping it needs a type change inside the plugin, keep the
  change there.
- **The hook path.** Check whether an allowing hook reaches `resolve_check` as
  `Decision::Allow { reason: Hook }`. If it does not, say what it is and record that instead.
- **Item time.** The clock assumes a turn's first item starts when the turn does. Check this
  against the kernel and say what you find.
