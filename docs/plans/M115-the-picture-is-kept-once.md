# M115 — The picture is kept once

## Goal

ADR-0067: share pictures in memory, store one content-addressed blob per
session, reference it in journal v2, estimate pixels and bound each request.
Live frames stay inline. This is M113's first version-line bump.

## Bricks, in build order

1. **Contracts first.** Commit v1 inline/v2 reference fixtures and red tests:
   equal replay, duplicate append → one blob, missing blob → text note,
   pixel estimate, newest-four budget, line size and old-checkpoint rejection.
2. **Shared Image.** `data: Arc<str>`; optional width/height, omitted when
   absent. Adapt all constructors and provider readers mechanically in one
   commit; run full gates before proceeding.
3. **Measured producers.** `bingo_pictures::bounded` retains decoded or
   produced dimensions; sniffed/accepted/load, Read and MCP pass them through.
4. **Estimate.** `ceil(width * height / 750)` with both dimensions, otherwise
   `IMAGE_TOKENS`; aggregate estimates and their anchor saturate.
5. **Request weight.** After ADR-0061's age rule, traverse newest first with
   `REQUEST_PICTURE_BYTES = 4 * 1024 * 1024` beside `IMAGE_ROUNDS_KEPT`;
   elide pictures that do not fit with the same note. Projection only.
6. **Store blobs.** `blobs::put/get` uses M114's bingo-digest SHA-256, temporary
   file + rename, skips an existing blob, and stores decoded bytes per session.
   A record codec with injected blob lookups rewrites only known content-part
   paths; RawValue preserves opaque payload bytes. Live Event stays inline.
   Journal VERSION = 2; prepare a v1 journal with a version line before append;
   the 1→2 migration is identity. Replay resolves references or the missing note.
   Checkpoints also store references; bump their independent codec to 2 so old
   checkpoints replay/rebuild and old readers cannot seek past the journal bump.
   Prepare the journal before checkpoint publication; cache writer preparation
   under the store mutex so appends and checkpoint resumes do not rescan history.
   Use existing workspace base64, serde/rc and serde_json/raw_value; no new crate.
7. Accept ADR-0067; dated amendments to ADR-0005 §8, ADR-0040 §3,
   ADR-0061 §1 and ADR-0062 §1.

## Files

- SDK `src/{model.rs,tokens.rs,store/checkpoint.rs}`, Cargo.toml and record fixtures
- Every `Image {` constructor; provider request builders (`rg -n 'Image \{' crates`)
- `bingo-pictures/src/**`, `bingo-tool-fs`, `bingo-mcp`
- Core `context/budget.rs`, `context/elide/images.rs` and projection consumers
- `bingo-store-jsonl/src/**`, fixtures, Cargo.toml and Cargo.lock
- `schema/{rpc,plugin}.json`: mechanical optional Image dimensions
- Mechanical Image fixtures in provider, core, print, TUI, MCP and RPC tests;
  dimension-only expectations in pictures, MCP, print, Read and tool-web body/fetch
- `crates/bingo/tests/cli/images.rs`: use journal replay in its fixture reader;
  preserve existing CLI assertions
- `docs/adr/{0067,0005,0040,0061,0062}*.md`

## Exit criteria

- [x] v1/v2 fixtures yield equal Images; appending to v1 writes the version
      line and still replays the whole journal.
- [x] Two identical pictures produce one blob.
- [x] A frame with minted SessionId/ItemId is <1 KiB before newline;
      3,000,000-byte and 1-byte pictures with the same dimensions/IDs differ
      only by `digits(3000000) - digits(1) = 6` bytes in line length.
- [x] Missing blob opens with the note; estimate and budget tests pass.
- [x] A pre-M115 binary refuses v2 with the newer-bingo message; paste output
      or the reason it cannot be built.
- [x] All gates (Windows C-header deferrals follow execution protocol §7).

## Non-goals

Wire references (M107 handles opted-in clients), blob collection beyond session
removal, and rewriting pictures already inline in v1 journals.

## Risks

- Re-verified at cc0b8572: image projection is context/elide/images.rs;
  thresholds remain in context/budget.rs; M113 STEPS is journal/migrate.rs.
  M114 checks only its checkpoint anchor, so the checkpoint codec must bump.
  M116 already runs store I/O on a blocking worker; base64/hash CPU stays there.
  Producers already pass bounded Images through; no bridge redesign needed.
- Pixel metadata can reach u32::MAX: regression tests require saturating sums.
- 2026-10-13 decision: replace the guessed 300-byte cap with <1 KiB and only the byte-count digit delta; ADR-0065 §1 and ADR-0067 §2 stand. Equivalent assertion refinements are pre-authorized.

## Verified (2026-10-13)

Contracts were committed before implementation; selected original red output:
```text
cargo test -p bingo-sdk pixel_contract --locked
  dimensions_set_the_picture_estimate ... FAILED (1600 versus 5334)
cargo test -p bingo-store-jsonl the_planned_v2_picture_line --locked
  under_one_kib ... FAILED: 4000358 bytes before newline
  length ... FAILED: left 3999996, right 6
cargo test -p bingo-store-jsonl a_version_one_checkpoint --locked
  FAILED: store.resume(...).state.is_none()
cargo test -p bingo-sdk -p bingo-core extreme_pixels --locked --no-fail-fast
  2 targets failed: attempt to add with overflow
```
Final gates ran one at a time. Full logs: `target/m115-verification/` (ignored).
```text
cargo fmt --all -- --check
  exit 0
cargo check --workspace --all-targets --locked
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.24s
cargo clippy --workspace --all-targets --locked -- -D warnings
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.51s
cargo test --workspace --locked -- --test-threads=1 --skip a_token_may_be_dialled_again_once_its_stream_has_closed --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing
  inline_and_blob_fixtures_resolve_to_the_same_image ... ok
  two_picture_appends_keep_one_blob_and_upgrade_the_journal ... ok
  deleting_a_blob_keeps_the_size_note_and_the_session_readable ... ok
  a_version_one_checkpoint_is_replayed_after_the_bump ... ok
  dimensions_set_the_picture_estimate ... ok
  six_megabyte_pictures_keep_only_the_newest_four ... ok
  aggregate_pixel_estimates_saturate ... ok
cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1
  test result: ok. 1 passed; 0 failed
cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
  test result: ok. 1 passed; 0 failed
cargo test -p bingo-store-jsonl the_planned_v2_picture_line --locked -- --nocapture
  picture lines: 432 / 438 bytes before newline
  test result: ok. 2 passed; 0 failed
scripts/check_discipline.sh
  discipline ok
scripts/budget.sh
  dependencies (unique, normal): 349 (max 349); warm core: 0s; isolation: 0
  target/debug: 8 GB (soft max 5); budget ok
cargo deny check
  advisories ok, bans ok, licenses ok, sources ok
scripts/tui-smoke.sh  # private TMUX_TMPDIR, removed afterward
  tui-smoke ok
```
Workspace totals: 5,112 passed, 0 failed, 4 existing ignored; both excluded ACP
cases passed alone (5,114 total). All 16 PTY tests passed. An earlier rewind
flake passed its immediate isolated rerun; dimension-only WebFetch fixtures
and the plugin schema were updated mechanically before the successful run.

Windows, sequential: `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`.
PASS: bingo-sdk, bingo-core, bingo-store-jsonl, bingo-provider-acp, bingo-surface-rpc.
CI deferred per §7: bingo-pictures, bingo-mcp, bingo-provider-anthropic,
bingo-provider-openai, bingo-surface-print, bingo-surface-tui, bingo-plugin-rpc,
bingo-tool-fs and bingo-tool-web: aws-lc-sys 0.44.0 lacks Windows SDK C headers.
bingo also fails in tree-sitter 0.26.13 and tree-sitter-bash 0.25.1; no Rust errors.

Compatibility: built a minimal reader executable using unmodified cc0b8572
SDK/store sources archived inside this worktree (no second worktree or branch).
It called the old JsonlStore::resume against an actual v1→v2 journal and v2 checkpoint:
```text
cargo run --manifest-path target/m115-old-source/Cargo.toml --target-dir target/m115-old-build -p bingo-store-jsonl --example m115_old_reader --offline -- target/m115-compat
[STORAGE] target/m115-compat/ses_1/journal.jsonl was written by a newer bingo (journal version 2, this build reads 1)
```
Temporary historical source/build trees were removed; this probes the old store
executable, not the full CLI. All exit criteria are met; Windows CI remains as above.
