# M143 — The desktop app: the shell and the wire

## Goal

ADR-0080 §1–5, §7 and §8. `apps/desktop` starts empty and becomes an npm
package (its own lockfile, not a cargo member) on Electron, React,
TypeScript and electron-vite. It spawns `bingo serve --stdio`, folds the
frames with the sdk's own `SessionState::apply` compiled to WebAssembly,
and draws Rev 6's shape: a project sidebar, a centred stage, a floating
composer island and the interaction panel. It names no plugin; CI
checks both per pull request, so a schema change that breaks it fails
that run. Release tags build it for macOS, Windows and Linux. After M113
(the golden fixtures), M123 (the naming rule it copies) and M133
(`bingo-schema-ts`, the one TS renderer, ADR-0080 §4).

## Bricks, in build order

1. **Contract: the sdk folds without a host.** `bingo-sdk` gains
   `default = ["host"]`, which turns on `tokio`, `tokio-util`,
   `async-trait` and `ulid` (now optional) and every module that holds a
   trait, handle, token or stream, or mints an id. The fold reaches
   tokio only via `view → host::Action` and
   `event → contributor::CONTRIBUTOR_PREFIX`; both move (to `view.rs`
   and `event.rs`) and are re-exported, so no path changes. First, the
   failing check: `cargo check -p bingo-sdk --no-default-features
   --target wasm32-unknown-unknown`. **If `SessionState::apply` still
   needs tokio after the cut (a folded type holds a token, stream or
   handle), stop and ask.**
2. **The boundary is wasm-bindgen with JSON strings.**
   `crates/bingo-fold-wasm` (`tier = "library"`, `cdylib` and `rlib`)
   depends on the sdk without defaults, `serde_json` and
   `wasm-bindgen = "=0.2.127"`. The pure `fold.rs` comes first, with
   native tests: `Fold { state }` with `new(summary)`, `from_state`,
   `apply(frames) -> Applied[]`, `head()` (no `items`), `item(id)` and
   `items()`. `lib.rs` wraps it in `#[wasm_bindgen]` (ADR-0080 §3).
   - Only `string` crosses, so the TS types come only from `schema/`.
     `scripts/budget.toml` rises by the measured host count (the member
     and the wasm-bindgen family), one line each.
   - `Applied` gains `Serialize` and `JsonSchema`; a
     `BINGO_UPDATE_SCHEMA` test commits `schema/fold.json` first. The
     renderer applies one batch per animation frame and reads `item(id)`
     on `Item`, `items()` on attach, `History` and `Lagged`, and `head()`
     otherwise. TS folds no copy of the state.
3. **The fold twin is the sdk's own fold.** A Rust test folds M113's
   `crates/bingo-sdk/fixtures/record/event/*.json`, in order, into
   `fixtures/folded.json`. A vitest folds the same frames through the
   wasm and asserts equal JSON. `scripts/build-fold.mjs` runs cargo for
   wasm32, then `wasm-bindgen --target web`. It reads the CLI version
   from `Cargo.lock` and refuses a mismatch. Its output is not
   committed.
4. **Codegen is M133's one renderer, `bingo-schema-ts`.** It gains a
   pure `zod.rs`, table-tested like its TS mode, still serde_json only.
   `bingo-fold-wasm`'s `tests/desktop_ts.rs` renders
   `schema/{rpc,fold}.json` with it into `apps/desktop/src/shared/rpc.ts`
   (types) and `src/main/rpc/validation.ts` (zod), and fails when either
   committed file differs (`BINGO_UPDATE_SCHEMA` rewrites both). Rei's
   `generate-rpc.mjs` does not come; its `rpc-validation.test.ts` comes
   as a vitest over the generated zod.
5. **The client's hardening, without fake hosts.** Rei's `rpc-client.ts`
   splits into `lines.ts`, a pure reader (16 MiB lines, split chunks, a
   truncated tail, bad JSON, the wrong protocol) tested over in-memory
   streams with `rpc-host.cjs`'s modes as table rows; `client.ts` (ids,
   32 pending, 32 MiB of writes, a 30 s timeout, `DesktopFailure`'s
   codes); and `process.ts` (a unix process group with Windows'
   `taskkill /T /F` beside it, `windowsHide`). Process tests run a real
   `bingo serve --stdio` under `BINGO_FAKE_SCRIPT` (`START_FAILED`,
   `PROCESS_EXITED`); escalation runs against a `node -e` child that
   never reads stdin.
6. **The interaction panel, from Rei.** `InteractionPanel.tsx` draws the
   pending permission, confirm, login, form or question and answers it
   on the wire. Rei tested it only via `App.test.tsx`, so each kind gets
   a test first. Permission shows the policy's words (modes: M144).
7. **The shell, Rev 6** (F05, F06, F07, F09). `package.json` pins Rei's
   exact versions and has a `dependencyReasons` map; a test fails a
   dependency with no line. `design/` is a copy of the Rev 6 document.
   Main spawns one `bingo serve --stdio` per project root (the packaged
   resource, then `BINGO_BIN`, then `PATH`) and forwards frames in order
   over the preload bridge. The renderer has `ProjectSidebar` (sessions
   from `session/list`), `Stage` (streaming items drawn from `display`
   and text) and `Island` (submit, `esc` to interrupt, a busy breath on
   its border). CSS tokens hold both palettes; springs stop under
   `prefers-reduced-motion`; the CSP adds only `'wasm-unsafe-eval'`.
8. **The check.** `check_discipline.sh` gains a section after M123's 4f:
   over `apps/desktop/src` (no tests, no generated files) it fails on a
   `bingo.`/`_bingo.` literal in any TS quote, a quoted 4c tool name, or
   a `rooms/` or `agent/` prefix. A seeded violation shows it failing.
9. **CI and release.** `ci.yml`'s `desktop` job pins node once, in
   `apps/desktop/.node-version`, and adds the wasm32 target and a cached
   `wasm-bindgen-cli` at the locked version. It runs `npm ci`,
   `typecheck` and `vitest` on ubuntu, macOS and Windows (the drift test
   runs in `test`). On ubuntu (`xvfb-run`) and Windows it builds `bingo`
   and runs the Playwright smoke: open a temporary project, submit, see
   the fake answer, allow a scripted permission, `esc` a second turn.
   `release.yml`'s `desktop` job runs after `build` for macOS arm64 and
   x64, Windows x64 and Linux x64. It places the matching `bingo` in
   `resources/` and builds a `dmg`, `nsis` or `AppImage`, stamped with
   the version from `cargo metadata`.

## Files

- `crates/bingo-sdk/{Cargo.toml,src/{lib,view,event,ids,model,host,contributor}.rs}`
- `crates/bingo-fold-wasm/{Cargo.toml,src/{lib,fold}.rs,tests/desktop_ts.rs,fixtures/folded.json}`, `schema/fold.json` (new)
- `crates/bingo-schema-ts/src/{lib,zod}.rs` (zod mode)
- `Cargo.toml`, `scripts/{budget.toml,check_discipline.sh}`, `.github/workflows/{ci,release}.yml`, `ARCHITECTURE.md`
- `apps/desktop/{package.json,package-lock.json,.node-version,.gitignore,tsconfig.{node,web}.json,electron.vite.config.ts,electron-builder.cjs,vitest.config.ts,playwright.config.ts}`
- `apps/desktop/{design/,scripts/{build-fold,version}.mjs}`
- `apps/desktop/src/main/{index,security}.ts`, `runtime/locate.ts`, `rpc/{lines,client,process,validation}.ts`
- `apps/desktop/src/{preload/index.ts,shared/{rpc,ipc}.ts,renderer/{index.html,main.tsx,App.tsx,fold/{load,store}.ts,theme/{tokens.css,motion.ts}}}`
- `apps/desktop/src/renderer/shell/{ProjectSidebar,Stage,Island,InteractionPanel}.tsx`
- Tests beside modules; `apps/desktop/tests/{smoke.e2e.ts,fixtures/hello.json}`

## Exit criteria

- [ ] Brick 1's wasm32 `cargo check` fails before and passes after
      (pasted). `cargo tree -p bingo-fold-wasm --target
      wasm32-unknown-unknown -e normal | rg tokio` finds nothing.
- [ ] `cargo test -p bingo-schema-ts` and `-p bingo-fold-wasm` pass;
      `fold.json`, `folded.json` and both desktop files regenerate
      unchanged, and `desktop_ts` fails once on a hand-edited `rpc.ts`
      (shown). `typecheck` and `test` pass, naming the fold twin.
- [ ] The smoke passes on macOS and CI's ubuntu and Windows;
      `electron-builder --dir` builds on macOS and CI's Linux.
- [ ] ``rg "['\"`]_?bingo\." apps/desktop/src --glob '!**/*.test.*'``
      finds nothing, and the desktop check fails on a seeded literal.
- [ ] `scripts/budget.sh` and `cargo deny check` pass, as do all gates
      and `cargo check -p bingo-fold-wasm --target x86_64-pc-windows-msvc`.

## Non-goals

- Onboarding (F01–F04) and Settings (F11), left to the CLI and M98's
  `settings.toml`; mod drawing (M144); Rei's panes (M145); signing and
  notarization (unsigned until the person supplies certificates).
- Rei's `--json-events`, `foldSessionFrame`, `sessionTree.ts`,
  `generate-rpc.mjs`, fake hosts and `zh-CN` (UI copy is English).

## Risks

- `#[wasm_bindgen]` expands to `#[unsafe(no_mangle)]` and unsafe blocks;
  lints are believed silent in an external macro, unverified. Brick 2
  opens with a one-function crate under `forbid(unsafe_code)`; if the
  lint fires, stop and ask.
- `ulid 3` pulls `getrandom`, which has no default wasm32 backend; the
  fold never mints, so `ulid` is host-only. `Timestamp::now` panics on
  wasm32; the fold never reads the clock. The twin catches either.
- Electron tests wait on selectors, never sleeps. The macOS x64 build is
  made on arm64, safe while no native module exists (`node-pty`: M145).
- Re-verify at start: M113's fixture path; M133's `bingo-schema-ts`
  and its TS mode's output; M121/M123's slots and check letter; `schema/rpc.json`'s
  methods; `bingo_provider_fake::SCRIPT_ENV`; what sits behind `host`;
  the wasm-bindgen lock; Rei's files (read only); macOS runner labels.
