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

- [x] Schema or fixture tests for each new field land before the code that fills it. The
      regenerated schemas and TypeScript types are committed.
- [x] A journal from before M201 replays unchanged. A journal with the fields round-trips.
- [x] Core tests, one per path:
  - an edit inside the cwd in `acceptEdits` records the mode;
  - any call in `bypassPermissions` records the mode;
  - a call allowed by a rule added with "yes for this session" records the rule and its
    session source;
  - a settings allow rule records its settings source;
  - a hook's allow records the hook;
  - a read-only call and an asked call record nothing on the call.
- [x] A permission ask and a question answered after a wait record `asked_at` equal to the
      interaction's `opened_at`.
- [x] Clock unit tests:
  - no asks;
  - one closed ask, subtracted;
  - an open ask freezes the clock at its `opened_at`;
  - a stopped turn;
  - a receipt without `asked_at` subtracts nothing;
  - the newest turn and an older one read alike.
- [x] Read tests: `N lines` for a whole read and for a partial read of the same file, and `W×H`
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

## Verified

- [x] Contracts precede producers: `6e0d5cc2` and `28ca9900`; schemas/types accompany their fields.
- [x] `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --lib --test why_and_time --locked` → exit 0; optional interruption timestamp round-trips; old item fixtures omit new fields.
- [x] Schema generators: plugin-rpc `--lib` → `132 passed`; surface-rpc `--lib schema` → `6 passed`; mods `--lib schema_and_types_are_generated_from_the_same_rows` → `1 passed` (each with `BINGO_UPDATE_SCHEMA=1`, `--locked`).
- [x] `cargo test -p bingo --test kernel provenance --locked` → three real-ladder reads failed before B1, then `3 passed`; bypass/settings/session reads have no reason.
- [x] `cargo test -p bingo-core -p bingo-permissions --lib --locked` → core `682 passed`; permissions `114 passed` in final targeted runs; deny/ask precedence and untrusted reads remain intact.
- [x] B2/B3: `cargo test -p bingo-core --lib session::tests::receipts --locked` → four failures before fixes; final core run passes all six receipt/stop/wait tests, with immediate-denial feedback preserved.
- [x] `cargo test -p bingo-sdk --test turn_clock --test why_and_time --locked` → `11 passed`, `2 passed`; interruption clock test failed before the fix. No production clock caller.
- [x] `cargo test -p bingo-tool-fs -p bingo-store-jsonl --lib --locked` → `91 passed`, `74 passed`; singular-line and original-dimensions tests both failed before fixes.
- [x] Store replay plants the recorded pre-M201 `frames-v1.jsonl`, folds five items, omits new fields, retains a 6,039 µs clock, and leaves journal bytes unchanged.
- [x] `node --test sdk/typescript/src/*.test.ts` → `tests 33`, `pass 33`, `fail 0`; deleted the vacuous provenance test, leaving plugin-kit's expected count unchanged.
- [x] `cargo fmt --all -- --check` → exit 0; `scripts/check_discipline.sh` → `discipline ok` (existing warnings).
- [x] `scripts/budget.sh` → `dependencies (unique, normal): 423 (max 423)`, `budget ok`; no dependency added; target/debug 15 GB exceeds the soft 5 GB limit.
- [x] `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
- [ ] `cargo check --workspace --all-targets --locked` → exit 101: TUI `transcript.rs:424` pattern must ignore the new field (E0027).
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 101, same pattern.
- [ ] `cargo test --workspace --locked` → exit 101, same pattern; no wall-clock test failure.
- [ ] `cargo test -p bingo --test cli --test rpc --test kernel --test plugin_kit --locked` → exit 101, same pattern; no black-box re-pins.
- Windows: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked` → sdk/core/store-jsonl/agents/checkpoints/context/rooms exit 0.
- Windows: permissions → tree-sitter headers; pictures/tool-fs/print/tui → aws-lc-sys headers; bingo → aws-lc-sys/rquickjs-sys headers (exit 101, accepted host limits).
- Shapes: `allowedBy: Option<Box<Reason>>` reuses policy provenance compactly; optional `Rule.source` distinguishes settings/session. Trusted reads retain ReadOnly unless a hook allows them.
- Shapes: optional `askedAt` on receipts/questions/interruptions preserves the opening instant; existing item time supplies the end. Absence is omitted; no duration is stored.
- B2 withdraws actor approval receipts and reconciliation: the gate writes one final receipt with the settling interaction and first opening time. Deny/question answers retain dev's actor writers; exact interaction-id dedup never rewrites receipts.
- Allow receipts again follow the ack; `a_permission_is_answered_once_and_late_answers_are_rejected` restores dev's expectation. An edited call asked again then denied retains that final ask's own opening time in the actor's Deny receipt.
- B3 replaces cancellation receipts with an interruption timestamp: the running turn holds the earliest open ask only until the actor stamps the interruption, including dropped turns. Explicit question Cancel still means skipped; interrupted asks add no answer to context.
- B4/clock helpers split marking Running, selecting items/end/waits/deadline; overlapping waits and skew clamping are unchanged.
- Read reports total lines (including partial reads), singular `1 line`, and original file dimensions using the existing header reader; model parts are unchanged.
- Deleted the one-reader reason fixture; core tests use an inline opaque mode, and the binary tests cover the real bypass ladder. Hook allows on reads still record Hook.
- Plan findings retained: legacy HookOutcome has no Allow; a full tool.check interceptor does. Input items precede TurnStarted slightly, so the clock follows the specified first-item start.
- Other surfaces unchanged: ACP bridge/shape::refused carries refusal text; channel-adapter question::ladder carries permission summaries; neither renders automatic allow provenance or turn duration. Feishu's footer uses usage.
- Scope: accepted TUI compatibility remains; new interruption literals receive None. One additional `{ marker, .. }` pattern is prepared in `/tmp/m201-interruption-pattern.patch`, awaiting the exact-scope exception requested from the user.
- Owed: apply that pattern, rewrite history once (including folding TUI literals into the field commit), then repeat full gates/black-box checks. No history rewrite has occurred during fixes.
