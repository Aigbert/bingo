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
   already agree. Run validate and committed parity behavior tests for all three ports.
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
  `src/{lib,imports,strip,load,mount,module,chain}.rs`, `src/dollar/{table,tests,callback}.rs`,
  `tests/{strip,engine,testing,plugin}.rs`: schema-derived data reads and erased imports.
- `schema/dollar.json`, `sdk/typescript/src/{dollar,dollar-types,plugin}.ts`,
  `crates/bingo-plugin-rpc/src/{wire,bridge,validate}.rs`, generated plugin schemas/wire,
  `docs/adr/0075-plugin-protocol-2.md`: plugin metadata in both transports.
- `crates/bingo/src/plugins/test/driver.rs`: test metadata parity.

## Exit criteria

- [ ] Each committed example validates/tests; new band/pane contracts fail.
- [ ] Original ports validate/test; diff evidence proves import/tool-map only.
- [ ] mods_parity passes including Node, required on Linux CI; incomplete.
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
  All originals read; tool inputs agree; Linux CI requires Node 24.17.0. Examples remain rewrites.
- Both authorized blockers fixed: schema-derived data reads preserve alias refusals; erased imports share the runtime boundary and never load d.ts.
  mount.rs folds Views, not sources; Engine/Runner share load.rs/strip.rs.
- STOP, protocol §6: the new port failures need additional Files/scope:
  M142 `validate/{keys,hooks}.rs` rejects object atom keys and constant
  matcher values; M138 SDK state helpers also require string atom keys.
  M136/M137 tool.call rejects deny-only results; M140 ui.status(undefined)
  does not clear status. Port tests reproduce denied edits running and a
  failure status remaining after success. No runtime fix attempted for these.
- Options: explicitly widen M146 to these validator/runtime contracts and
  tests, or leave parity incomplete. No live milestone owner remains.
  New kernel doors/nouns or changed Decisions still require a stop.
- Source-inspected, not yet closed: numeric clock.now, callback-free command
  registration (M138), synchronous ui.resolve (M140). Missing catch/120 s
  tests and the band/pane behavior implementations remain pending.

## Verified (2026-10-14, incomplete; stopped again)

No live/credential tests, foreign processes, terminal apps or tmux runs.
`cargo test -p bingo-mods --lib noun_data_reads_are_values_but_noun_and_method_aliases_are_not --locked`:
red `FAILED. 0 passed; 1 failed` (data reads rejected as dollar-static), then
`cargo test -p bingo-mods --lib validate::tests --locked`: `6 passed; 0 failed`.
The import contracts then failed: `Is a directory (os error 21)`;
`FAILED. 6 passed; 2 failed`. After the shared resolver: `8 passed; 0 failed`.
Each contract was committed before its fix(mods) implementation.
`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods --lib schema_and_types_are_generated_from_the_same_rows --locked`: `1 passed; 0 failed`.
`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc --lib --locked`: `130 passed; 0 failed`.
`cargo test -p bingo-mods --locked`: exit 0, 75 passed, 1 manual RSS test ignored.
Mechanical fixture: tests/plugin.rs now expects manifest version in identity.
`cargo build -p bingo --locked`: exit 0, Finished dev profile in 10.10s.
`cargo test -p bingo --test mods_parity port_layout_changes_only_the_package_specifier --locked`:
red while rewrite-only tests were copied; after selection fix `1 passed; 0 failed`.
Disposable-copy `bingo plugins test` results (all original tests retained):
```
tool-call: new environment guard and shell status tests failed first;
then 4 PASS (environment guard, envrc, shell status, original rewrite).
band: new test FAIL (no per-turn count); original rewrite PASS.
pane: new test FAIL (tool-calls not opened); original rewrite PASS.
```
Original input: supplied `980b9fc1-a2b6-4a4e-a594-9bd988be2aca/scratchpad/cc-examples-2.1.296/`.
`scripts/port_examples.sh "$SOURCE" "$M146_PORTS"`: exit 1, temporary output only.
Source-free diff evidence: tool-call.ts -1/+1, band.tsx -2/+2,
band-state.d.ts -1/+1, pane.tsx -2/+2, pane-state.d.ts -1/+1;
each reports `package specifier only; source unchanged`. No tool map needed.
```
tool-call validate: exit 0; test: exit 1 (envrc PASS; deny/status FAIL)
hooks/band.tsx:6:14 state-key state key must be a string literal
hooks/band.tsx:7:18 state-key state key must be a string literal
band validate: exit 1; test: exit 1 (AbovePrompt engine fallback)
hooks/pane.tsx:7:15 state-key state key must be a string literal
hooks/pane.tsx:37:3 on-literal on matcher must be literal
pane validate: exit 1; test: exit 1 (tool-calls not opened)
```
The script now runs only committed *.parity.test.ts on originals; independent
rewrite-only behavior tests still run on the committed examples.
Gate commands/results: remaining gates not run after the protocol stop.
- `cargo fmt --all -- --check`: prior-run pass; formatting applied this run.
- `cargo check --workspace --all-targets --locked`: not run.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: not run.
- `cargo test --workspace --locked`: not run; new band/pane contracts fail.
- `scripts/check_discipline.sh`: not run.
- `scripts/budget.sh`: not run; no new dependencies.
- `cargo deny check`: not run.
- `scripts/tui-smoke.sh`: not run.
- `cargo check -p bingo -p bingo-mods -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc`: not run.
Temporary ports/copies cleaned; originals unchanged. No logs/captures committed.
No matrix awk/rg counts, Node parity or TUI evidence claimed.
