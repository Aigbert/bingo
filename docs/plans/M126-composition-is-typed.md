# M126 — Composition is typed

## Goal

ADR-0069 §5–6. The bin hands each surface a JSON object,
`SurfaceOptions.args`, whose eight keys it writes in
`crates/bingo/src/main.rs` (`surface_options`: `outputFormat`,
`inputFormat`, `permissionPromptTool`, `noPrintOnExit`, `updateCheck`,
`images`, `pictureCacheDays`, `measure`; `serve_options`: `transport`)
and that print, rpc and the TUI read back by name. Before a host exists
it settles settings by walks of its own (`update_wanted`, `tui_measure`,
`demo_ui`: the highest layer naming a key wins; `channels_wanted`:
`.any()`), while the kernel deep-merges. It re-spells two plugins'
schemas: `bingo/src/mcp.rs` (`"bingo.mcp"`, `"mcpServers"`) and
`provider.rs` (`INSTANCE_KEYS`, `instances`, `baseUrl`). A plugin is
switched four ways.

After this milestone each surface takes its own `Options`, the bin reads
only what decides the composition and reads it through the kernel's
merge, every other key is read by the plugin that claims it, and the bin
imports plugin keys instead of spelling them. No wire change, no new
dependency.

## Bricks, in build order

1. **Contract tests first**, failing:
   - a settings table through `bingo_core::settings::merge(layers,
     claims)` with `Claim::from_manifest` over the static manifests: a
     higher layer's `channels` turns chat off; the demo is composed on
     `enabledPlugins["bingo.demo.ui"]: true` or `--demo-ui`, and not when
     the key is absent; legacy `demoUi: true` still composes it;
   - the TUI reads `tui.measure` and `update.check` from its slice at
     `register`, through the deep merge;
   - `bingo mcp add` writes what `bingo_mcp::Settings` reads, and
     `bingo provider add` writes what each provider's `instances.rs`
     reads, both in `crates/bingo/tests/cli`.
2. **`SurfaceOptions.args` is deleted** (`bingo-sdk/src/surface.rs`).
   It keeps `cwd`, `selector`, `prompt` and `env`. Every construction
   (the bin's three, channels, the tests) loses the member.
3. **Each surface owns its options.** Each is `Default`, for
   compositions that run no surface (`provider_ids`, `bingo plugins`):
   - print: `Options { output_format, input_format,
     permission_prompt_tool, images }`. The three enums and their words
     live in print and implement `FromStr`; `Cli` parses into them and
     lists their values from print's constants.
   - rpc: `Options { transport }`; `transport(&opts.args)` goes.
   - TUI: `Options { print_on_exit, picture_cache_days }`.
   `TuiPlugin::new`, `PrintPlugin::new` and `RpcPlugin::new` take them,
   and `plugins()` takes a `Composition` carrying the three.
4. **The TUI reads its own keys.** It claims `tui` and `update`
   (`lib.rs`) and reads both slices at `register` with
   `Registrar::config::<T>()`, keeping them for `run`.
   `settings::given`, `run/updates.rs`'s `args` read and
   `showing::cache_days(&opts.args)` go; the cache days come from
   `Options`, filled from `KernelSettings` (`picture_cache_days`).
5. **The bin resolves once.** `Decided`, `update_wanted`,
   `tui_measure`, `demo_ui` and `channels_wanted`'s walk are deleted.
   The bin merges the layers with the static manifests' claims and
   reads two answers: whether the demo is composed, and whether chat
   starts (`bingo_channels::wanted` over the merged `channels` slice, or
   `--channels`). `BINGO_FAKE_SCRIPT` stays an environment variable.
6. **One switch.** `enabledPlugins` is the only key that switches a
   plugin (ADR-0057). For one release:
   - `demoUi: true` composes the demo, and the demo posts a notice
     naming `enabledPlugins` when its slice holds `demoUi`;
   - `experience.enabled: false` keeps working as it does at
     `register`, with the same notice.
   No settings file is rewritten. A dated line in each plugin says when
   the legacy read goes.
7. **The bin stops re-spelling.** `bingo/src/mcp.rs` imports the
   manifest id and `SETTING` from `bingo-mcp`. `provider.rs` builds
   `INSTANCE_KEYS` from the provider crates' key constants (openai owns
   `openai` and `codex`) and writes an instance through that crate's
   endpoint type (`OpenAiEndpoint` and its anthropic twin), so
   `baseUrl` is spelled once, where `instances.rs` reads it.
8. **Records.** ADR-0069 is accepted for §5–6. ADR-0003 §2 and §4,
   ADR-0057 and ADR-0043 §4 get dated notes. The settings guide pages
   say `enabledPlugins` for the demo and experience.

## Files

- `crates/bingo-sdk/src/surface.rs`
- `crates/bingo-surface-print/src/{lib,input,render}.rs`
- `crates/bingo-surface-rpc/src/lib.rs`
- `crates/bingo-surface-tui/src/{lib,run,settings}.rs`, `run/{showing,updates}.rs`
- `crates/bingo-channels/src/{lib,settings}.rs`
- `crates/bingo-demo-ui/src/lib.rs`, `crates/bingo-experience/src/lib.rs`
- `crates/bingo-mcp/src/lib.rs`, `crates/bingo-provider-{openai,anthropic}/src/{lib,instances}.rs`
- `crates/bingo/src/{main,mcp,provider,plugins}.rs`, `crates/bingo/tests/cli/`
- every guide page that names `demoUi` or `experience.enabled`
- `docs/adr/{0003,0043,0057,0069}-*.md`

## Exit criteria

- [ ] The settings table test passes; the failing first run is pasted.
- [ ] `rg 'opts\.args|\bargs: json!|pub args' crates/bingo-sdk/src crates/bingo-surface-print/src crates/bingo-surface-rpc/src crates/bingo-surface-tui/src crates/bingo/src`
      finds nothing.
- [ ] `rg 'fn (update_wanted|tui_measure|channels_wanted|demo_ui)\b|struct Decided' crates/bingo/src`
      finds nothing.
- [ ] `rg '"bingo\.mcp"|"mcpServers"|"baseUrl"' crates/bingo/src`
      finds nothing.
- [ ] CLI black-box tests pass unchanged for `--print` with every
      `--output-format`, `--input-format stream-json`,
      `--permission-prompt-tool stdio` and `--image`, and for
      `bingo serve --stdio`: exit status, stdout purity, NDJSON validity.
- [ ] A black-box test: `demoUi: true` composes the demo and writes the
      notice to stderr; with no key the demo is absent from
      `bingo plugins`.
- [ ] `scripts/tui-smoke.sh` passes, and a tmux drive starts the TUI with
      `tui.measure` and `update.check: false` in a project layer and
      shows both honoured.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the bin and the three surfaces.

## Non-goals

- `BINGO_FAKE_SCRIPT` as a setting; it selects a harness.
- External surfaces over plugin-rpc, and `uses` in `plugin.json` (M132).
- New settings keys, a settings file format change, or a migration that
  rewrites a person's files.

## Risks

- The deep merge changes two answers: a higher layer can now turn chat
  off, and `channels` objects in two layers merge instead of either
  sufficing. Say both in the commit body and in the channels guide.
- `--help` must still list each flag's values. Print's constants feed
  `possible_values`; a CLI help snapshot pins it.
- The demo was on by default nowhere, but a person with `demoUi: true`
  must see one release of notice before it stops working.
- Re-verify at start: the `main.rs` functions above by name, M119's
  `uses` and constructor shape, `Registrar::config` (`sdk/plugin.rs`),
  `Claim::from_manifest` (`core/settings.rs`), each provider's
  `instances.rs` and which crate owns `codex`, and that no plugin-rpc
  message carries `SurfaceOptions`.
