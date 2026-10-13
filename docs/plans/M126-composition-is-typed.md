# M126 — Composition is typed

## Goal

Implement ADR-0069 §5–6: surface-owned typed options; one settings merge;
plugin-owned settings keys and endpoint shapes. No wire change or dependency.

## Bricks, in build order

1. Commit regression contracts: composition matches merged settings, higher
   layers clear channels, explicit demo switches compose it, TUI registration
   reads its slice, and CLI writers produce settings their plugins read.
2. Delete `SurfaceOptions.args`; retain cwd, selector, prompt and env.
   Update constructors and fixtures mechanically (protocol step 6).
3. Print, RPC and TUI own `Options`, passed to their plugin constructors.
   Print owns the CLI enums, `FromStr` implementations and possible values.
   A bin `Composition` carries the three options; unused surfaces use defaults.
4. TUI reads its claimed `tui` and `update` roots at registration. Measure
   and update checks come from that slice; cache days come from typed options.
5. Resolve composition using `settings::merge` and static manifest claims.
   Remove `Decided` and the bin's settings walks. `KernelSettings` gains
   picture cache days, with both existing spellings normalized before merging.
6. `enabledPlugins` is the switch. Demo remains opt-in (`true` or `--demo-ui`).
   Keep `demoUi` and `experience.enabled` for one release; their plugins emit
   startup notices naming the replacement. Never rewrite settings for this.
7. Bin imports MCP's manifest/key and provider key constants; provider add
   serializes provider-owned endpoint types, omitting default/absent fields.
8. Accept ADR-0069 §5–6; date notes in ADR-0003 §2/§4, ADR-0043 §4 and ADR-0057.
   Update the experience and channels guides.

## Files

- `bingo-sdk/src/surface.rs`; `bingo-core/src/settings{,/merge}.rs`.
- Print `lib`, `input`, `render`, `options`, `hosted`, `stream_json`;
  RPC `lib`; TUI `lib`, `settings`, `run`, `run/{showing,updates}`, `doubles`.
- Channels host test constructors and guide; demo/experience `lib` and guide.
- MCP `lib`; provider-openai/anthropic `lib`, `settings`, `instances`.
- Bin `main`, `composition`, `mcp`, `provider`, `plugins/`, `tests/cli/`.
- ADRs 0003, 0043, 0057, 0069. Paths above are under `crates/`.

## Exit criteria

- [x] Composition settings table passes; failing first run is pasted.
- [x] No `SurfaceOptions.args` or surface `opts.args`; broad `pub args` /
  `args: json!` matches only existing command/action arguments.
- [x] No `Decided`, `update_wanted`, `tui_measure`, `channels_wanted`, `demo_ui`
  functions in the bin, nor literal `"bingo.mcp"`, `"mcpServers"`, `"baseUrl"`.
- [x] CLI black-box tests preserve all print formats, stream-json input,
  stdio permission prompts, images and `serve --stdio`; help pins values.
- [x] Legacy demo composes and reports `enabledPlugins` on stderr; absent
  demo is not listed; the new switch lists it.
- [x] TUI smoke passes and an owned tmux drive proves project-layer measure
  and disabled update checks are honoured.
- [x] Upgrade gates pass; every touched crate receives an MSVC check,
  deferring missing third-party C headers to Windows CI per protocol.

## Non-goals

Other milestones, external surfaces, live/credential tests, new dependencies,
settings-file rewrites, and changing `BINGO_FAKE_SCRIPT` into a preference.
Never push, merge, open a PR, or touch another run's tmux/processes.

## Risks

- Higher layers can clear channels; channel objects merge field by field.
  The channels guide and implementation commit explain both changes.
- Empty picture objects now preserve lower leaves, consistent with the kernel
  merge; explicit null still clears. Both cache-day spellings retain precedence.
- Keep TUI `run.rs` changes to option plumbing; M135 edits that file too.
- Broad args searches also find SDK command/action args and M123's slot test;
  those are not surface options and stay. No schema message carries them.

## Re-verified (2026-10-13, f9eb788a)

Committed before implementation as `89a0be07`. Clean branch
`m126-composition-is-typed`; M108–M125 and M127–M134 are present.
M134 ACP remains an agent. M130 `wanted` takes `Platforms`. TUI already
claims both roots, but did not read them at registration. Kernel settings
had no cache-days field. Endpoint types live in provider `settings.rs`;
OpenAI owns codex. `plugins` is now a directory. No plugin-rpc message carries
`SurfaceOptions`. Mechanical constructor/import updates include channels
host tests, TUI doubles/run tests and print hosted/stream-json tests.
Use `BINGO_UPDATE_SCHEMA=1` for any generated fixture, including M133 TS wire.

## Verified (2026-10-14)

First red runs (`a4190d35`), trimmed:
```text
cargo test -p bingo --bin bingo composition_matches_the_merged_settings_table --locked
assertion `left == right` failed: demo: ... enabledPlugins ... bingo.demo.ui ... true
  left: false
 right: true
cargo test -p bingo-surface-tui registration_validates_the_claimed_settings --locked
assertion failed: TuiPlugin.register(&mut registrar).is_err()
```
Final native gates ran separately, in order. No live/credential-backed tests:
```text
cargo fmt --all -- --check
exit 0 (no output)
cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.57s
cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.67s
cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed
exit 0; summed results: 5150 passed; 0 failed; 4 existing ignored
CLI: test result: ok. 258 passed; 0 failed; 0 ignored
PTY: test result: ok. 16 passed; 0 failed; 0 ignored
TUI: test result: ok. 1123 passed; 0 failed; 2 ignored
cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
test result: ok. 1 passed; 0 failed
cargo test -p bingo-provider-acp --test bridge token --locked -- --test-threads=1
test result: ok. 3 passed; 0 failed
scripts/check_discipline.sh
discipline ok
scripts/budget.sh
dependencies (unique, normal): 355 (max  355)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 5 GB (soft max  5)
budget ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```
The first workspace run found a missing `list` in the new demo test and a
room test that passed alone; the final serialized run passed both. The next
run found the guide’s legacy-example check; documenting the supported legacy
spelling fixed it without changing that test. No PTY retries were needed.

MSVC: ran `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`
separately for all 12 touched crates (names below omit `bingo-`):
```text
core, demo-ui, experience, sdk, surface-rpc: PASS
bingo: DEFER aws-lc-sys, tree-sitter, tree-sitter-bash; stdio.h/stdlib.h/windows.h missing
channels, mcp, provider-anthropic, provider-openai, surface-print, surface-tui:
DEFER aws-lc-sys; stdlib.h/windows.h missing
```
Seven C-header checks remain for Windows CI (step 7); no Rust errors.
Removal searches found only existing command/action args and the slot test.

`scripts/tui-smoke.sh` ran under a new private `TMUX_TMPDIR`: `tui-smoke ok`.
An owned tmux settings drive printed the following (middle four rows omitted):
```text
project tui.measure=30 overrides user measure=96:
⏺ m12600 m12601 m12602 m12603
  m12620 m12621 m12622 m12623
project update.check=false overrides user true: no update.json stamp
owned tmux settings drive: PASS
```
Owned tmux servers and temporary homes were removed. No wire/schema fixture changed.
No push, merge or PR; stop at M126. Only approved Windows C-header deferrals remain.
