# M146 — Parity: Claude Code's examples run in the TUI

## Goal

The Phase 7 acceptance test: Claude Code 2.1.296's tool-call, band and
pane mods run in process in the TUI with only import-specifier and tool
name/input changes. Committed examples remain bingo's independent rewrites.
Every parity matrix row has evidence or an owned gap. ADRs: 0078, 0079, rejected 0080, and 0081 §7. No desktop work (M143–M145).

## Bricks, in build order

1. Check the committed examples are rewrites, not proprietary copies.
   Retain their existing tests; never commit original source without terms.
2. Add missing `bingo/testing` behavior cases, seen failing before code:
   - tool-call: deny Edit of `.env` and `config/.env.local`, naming the
     plugin; allow `.envrc`; trim Bash; failure status clears on success;
     a throwing hook's catch denies only before next was called.
   - band: count per turn; mock.clock past 120 s toasts once; mount
     AbovePrompt; button hides the summary, next prompt resets the count.
   - pane: session.start registers/opens tool-calls; list every call,
     dim completed calls, and bound the list by viewport.rows.
3. `scripts/port_examples.sh <originals> <temporary-output>` creates
   plugin.json, hooks/<original>, and types/index.d.ts from *-state.d.ts.
   Only claude-code → bingo is replaced; Edit/file_path and Bash/command
   already agree. Run validate and the committed tests for all three ports.
   Report failures to their owners (0078 §1/8, 0079 §1/4/5); never patch
   the originals or a Decision. Paste source-free diff evidence.
4. Drive a detached tmux session on an owned `-L` socket with isolated
   home/cwd and BINGO_FAKE_SCRIPT: protected edit, padded Bash, one failure.
   Open the waiting pane by the person's command, then capture denial,
   status, band, tool row and the pane inline at 160 and 100 columns (0081 §7).
5. Run tool-call under Node through the SDK with equivalent results.
   plugins test requires a module manifest; use binary black-box coverage
   for the process transport. Node type stripping does not run JSX ports.
6. Add Status to parity §2/3 (`done — evidence`, `gap — non-goal in M<n>`,
   or `n/a`), evidence lines to §4–6, and missing §8 owner Non-goals in a
   docs(plans) commit. Run the sequential protocol gates and record results.

## Files

- `examples/plugins/{tool-call,band,pane}/`: rewrites and behavior tests.
- `scripts/port_examples.sh`: temporary layout and validation/test runner.
- `crates/bingo/tests/mods_parity.rs`: CLI and Node parity coverage.
- `docs/design/mods-parity-2026-10.md`: matrix evidence.
- Owner plans, only to record missing §8 Non-goals (brick 6).

2026-10-14: delegate widened scope to both validator blockers and shared loading/schema paths:
- `crates/bingo-mods/src/validate/{dollar,source,tests}.rs`,
  `src/{strip,load,mount,module,chain}.rs`, `src/dollar/{table,tests,callback}.rs`,
  `tests/{strip,engine,testing}.rs`: schema-derived data reads and erased imports.
- `schema/dollar.json`, `sdk/typescript/src/{dollar,dollar-types,plugin}.ts`,
  `crates/bingo-plugin-rpc/src/{wire,bridge}.rs`, generated plugin schemas/wire,
  `docs/adr/0075-plugin-protocol-2.md`: plugin metadata in both transports.
- `crates/bingo/src/plugins/test/driver.rs`: test metadata parity.

## Exit criteria

- [x] Each committed example's validate and test commands exit 0 (below).
- [ ] Original ports validate/test; diff evidence proves import/tool-map only.
- [ ] mods_parity passes including Node, required on Linux CI. Two current
      tests pass; missing behavior and Node parity coverage remain unfinished.
- [ ] tmux captures: denial, status, band, tool row, inline pane at both widths.
- [ ] Every §2/3 row has Status; awk prints 0 rows without it.
- [ ] rg locates each §8 gap in its owner's Non-goals; all gates pass.

## Non-goals

- Client modules and ui.blit (M140/M141): no surface runtime/state copy.
- prompt.edit per keystroke (M136): surface composer/IME ownership.
- prompt.attachment/mention (M136): surfaces resolve @ before prompt.submit.
- ui.scroll/focus/selection/copy and open.isPlaced (M140/M141): surface
  ownership, no host verb toward a surface (ADR-0013 §4).
- tool.check without running, prompt.compose as a read, session.repo
  (M138): each needs a new kernel door.
- audio (M139): no player or audio stack; classic.* (M136): hooks-shell.
- Desktop/vscode/mobile surfaces: outside this tree (ADR-0080 rejected).
- Unasked/width-rule pane docking: person's opening and declared placement
  govern it (ADR-0081 §7, M151). telemetry/attribution.text: not applicable.
- Committing originals, JSX under Node, or type-checking Claude-only props
  such as AbovePrompt.hasSurvey (undefined means no survey here).

## Re-verified / Risks (2026-10-14)

- Base `3b10b522`; M136–M142 and M147–M151 landed; M143–M145 dropped.
  All five originals read; Edit/file_path and Bash/command already agree.
  Linux CI pins Node 24.17.0 and requires it. Committed examples are rewrites.
- Previous stop: validator misclassified plugin data reads and traversed
  erased imports. Delegate authorized fixing both here; M142 has no owner.
- Remaining candidate gaps: object atom keys (M142), numeric clock.now and
  callback-free command.register (M138), synchronous ui.resolve (M140),
  tool.call deny (M136). Reproduce; never change originals or Decisions.
- No new dependency, kernel door/noun or theme allowlist widening. Scoped
  schema changes use BINGO_UPDATE_SCHEMA=1 and a dated ADR-0075 line.
- Existing rewrite tests have different expectations. Brick 2 still needs
  behavior coverage; old port test failures alone do not prove runtime gaps.

## Verified (2026-10-14, incomplete; resumed)

`cargo build -p bingo --locked`: exit 0, Finished dev profile in 1m 02s.
All plugin commands used disposable homes; no live/credential tests.
`cargo test -p bingo --test mods_parity port_layout_changes_only_the_package_specifier --locked`
failed first: `scripts/port_examples.sh: No such file or directory`;
`test result: FAILED. 0 passed; 1 failed` (contract committed before runner).
`cargo test -p bingo --test mods_parity --locked -- --nocapture`: exit 0:
```
tool-call validate: exit 0     tool-call test: exit 0
band validate: exit 0          band test: exit 0
pane validate: exit 0          pane test: exit 0
test result: ok. 2 passed; 0 failed; 0 ignored
```
`SOURCE` was the supplied absolute `.../980b9fc1-a2b6-4a4e-a594-9bd988be2aca/scratchpad/cc-examples-2.1.296/`.
`scripts/port_examples.sh "$SOURCE" "$M146_PORTS"`: exit 1; output under
`/private/tmp/bingo-m146-ports.*/`, never the repository. Source-free diffs:
```
tool-call.ts: diff -U0: -1/+1 lines; package specifier only; source unchanged
band.tsx: diff -U0: -2/+2 lines; package specifier only; source unchanged
band-state.d.ts: diff -U0: -1/+1 lines; package specifier only; source unchanged
pane.tsx: diff -U0: -2/+2 lines; package specifier only; source unchanged
pane-state.d.ts: diff -U0: -1/+1 lines; package specifier only; source unchanged
hooks/tool-call.ts:8:20 dollar-static $ members cannot be aliased or stored
hooks/tool-call.ts:11:40 dollar-static $ members cannot be aliased or stored
band/pane validate: Is a directory (os error 21)
tool-call/band/pane validate: exit 1; tool-call/band/pane test: exit 1
```
Test result excerpts: `test requires an unanswered interaction` (tool-call),
`expected "⟨AbovePrompt⟩" to match "A small band above your prompt"` (band),
`unknown command: /ledger` (pane). These are old rewrite expectations.
The runner's first attempt had mismatched manifest/directory names for
band/pane; corrected and its contract rerun, then the above results obtained.
Gate commands/results (remaining gates not run after the protocol stop):
- `cargo fmt --all -- --check`: exit 0 before final port-directory correction;
  `cargo fmt --all` applied after that correction.
- `cargo check --workspace --all-targets --locked`: not run.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: not run.
- `cargo test --workspace --locked`: not run (targeted result above).
- `scripts/check_discipline.sh`: not run.
- `scripts/budget.sh`: not run.
- `cargo deny check`: not run.
- `scripts/tui-smoke.sh`: not run; no tmux or terminal app launched.
- `cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc`: not run.
Temporary ports were cleaned; originals kept. No captures/logs committed.
No matrix awk/rg counts or TUI evidence claimed.
