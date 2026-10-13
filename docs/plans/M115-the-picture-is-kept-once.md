# M115 — The picture is kept once

## Goal

ADR-0067. Pictures are shared in memory, stored once per session as
content-addressed blobs, referenced from journal version 2, estimated
from their pixels, and bounded per request. The wire stays as it is.
This is the first use of M113's version line.

## Bricks, in build order

1. **Fixtures and tests first.**
   - A v2 record fixture of a user item with a picture reference
     (`{mediaType, blob, bytes, width, height}`), and a v1 fixture with
     inline `data`; both must read into the same in-memory `Image`.
   - Store test: append a frame with a picture twice (two frames, same
     bytes) → one blob file, two references; replay yields `data`.
   - Missing-blob test: delete the blob, replay → a text part
     `[picture missing: image/png 1.2 MB]`, session opens.
   - Estimate test: a 2000×2000 picture with dimensions costs
     `ceil(4e6/750)` tokens; one without costs `IMAGE_TOKENS`.
   - Budget test: six 1 MB pictures in history → only the newest four
     are sent; older ones carry ADR-0061's elision note.
2. **`Image` changes shape in memory.** `data: Arc<str>`, plus
   `width: Option<u32>`, `height: Option<u32>` (skipped when `None`, so
   the wire form of a picture without them is unchanged). Every
   provider's request builder reads `&*data`; every constructor fills
   `Arc::from`. Mechanical; one commit.
3. **Producers fill dimensions.** `bingo_pictures::bounded` returns the
   dimensions it decoded or produced; `sniffed`, `accepted`, `load`, the
   `Read` tool and the MCP bridge pass them into `Image`.
4. **The estimate.** `bingo-sdk/src/tokens.rs`: `ceil(w·h/750)` when both
   are known, else `IMAGE_TOKENS`.
5. **The request budget.** In `bingo-core/src/context/budget.rs` (where
   `IMAGE_ROUNDS_KEPT` lives), `REQUEST_PICTURE_BYTES = 4 * 1024 * 1024`;
   the projection that applies ADR-0061's age rule then keeps pictures
   newest first until the budget is spent and elides the rest with the
   same note. Projection only; nothing recorded.
6. **The store writes blobs.** `bingo-store-jsonl`:
   - `blobs.rs`: `put(dir, bytes) -> sha256` (hashed by M114's
     `bingo-digest`; temp file + rename, skip if present),
     `get(dir, sha) -> Option<Vec<u8>>`;
   - append: for each picture part, decode base64, `put`, and write the
     line in v2 shape; this is the store's record type, distinct from
     `Event` (ADR-0065: the record is not the live type) — a
     raw-JSON record codec with blob lookups injected. Keep opaque payloads
     in RawValue (a Value round trip would normalize their bytes);
   - `VERSION = 2`; the first append to a v1 journal writes the version
     line first (M113's writer); the 1→2 step is the identity.
   - replay resolves references, or the missing-blob text part.
   - The checkpoint (M114) stores references, not inline pictures. Bump its
     independent codec version to 2: old checkpoints must replay and rebuild,
     and old readers must reject new checkpoints before seeking past a bump.
   - Preserve opaque record bytes; transform only known content-part paths.
   - Use the existing workspace `base64` dependency in the store for blob I/O.
7. ADR-0067 accepted; dated notes on ADR-0005 §8, ADR-0040 §3,
   ADR-0061 §1, ADR-0062 §1.

## Files

- `crates/bingo-sdk/src/{model.rs,tokens.rs}` and every `Image {`
  constructor (`rg -n 'Image \{' crates`)
- `crates/bingo-provider-*/src/**` request builders
- `crates/bingo-pictures/src/**`, `crates/bingo-tool-fs`, `crates/bingo-mcp`
- `crates/bingo-core/src/context/budget.rs` and the image projection
- `crates/bingo-store-jsonl/src/**` and its journal/blob fixtures
- `crates/bingo-sdk/src/store/checkpoint.rs` and checkpoint fixtures/tests
- `crates/bingo-sdk/Cargo.toml` (`serde/rc` for Arc), store Cargo.toml and Cargo.lock
- `schema/{rpc,plugin}.json` (mechanical optional Image dimension fields)
- `crates/bingo-sdk/fixtures/record/` and record contract tests
- Mechanical Image/schema fixtures in provider, core, print, TUI, MCP and RPC tests
- `crates/bingo/tests/cli/images.rs`: resolve stored pictures through journal replay
  in its fixture reader; keep the existing CLI assertions unchanged
- Dimension-only expected Image fixtures in pictures, MCP, print and Read tests
- `docs/adr/{0067,0005,0040,0061,0062}*.md`

## Exit criteria

- [ ] v1 and v2 fixtures read into equal `Image`s; a v1 journal appended
      to by this build carries a version line and still replays whole.
- [ ] Two identical pictures → one blob (test pasted).
- [ ] A picture frame with minted SessionId/ItemId is under 1 KiB before
      newline; 3 MB and 1-byte pictures with identical dimensions produce
      line lengths differing only by the decimal byte-count digit delta.
- [ ] Missing blob opens with the note; estimate and budget tests pass.
- [ ] An older binary (build `dev` before this milestone in a temporary
      worktree) refuses the v2 journal with the newer-bingo message —
      pasted; skip with the reason if building it is not possible.
- [ ] All gates.

## Non-goals

- Pictures by reference on the wire (M107's transport covers opted-in
  clients). Blob garbage collection beyond deleting the session.
- Re-encoding pictures already in v1 journals.

## Risks

- Changing `Image.data`'s type touches many crates; do it as its own
  commit with no behaviour change and run the full gates before
  continuing.
- Decoding base64 to hash it on every append costs CPU proportional to
  picture size; it runs on the store's blocking thread (M116 puts the
  writer there; before M116, measure that a 1 MB picture's append stays
  well under a frame interval and note it).
- Re-verified at `cc0b8572`: projection is `context/elide/images.rs`;
  `IMAGE_ROUNDS_KEPT` remains in `context/budget.rs`. M113 migration steps
  are in `journal/migrate.rs`; append_version is reserved for this bump.
  M114 checkpoint.rs and its witness verify only the anchor, so its codec
  must advance too. M116 already runs store I/O on a blocking worker.
  Producers already pass bounded Images through; no bridge redesign needed.

- 2026-10-13 decision: replace the guessed 300-byte limit with the revised size assertions above; keep ADR-0065 §1 and ADR-0067 §2 unchanged.

- 2026-10-13 decision: allow exactly the byte-count digit delta; assertion-only refinements preserving intent are authorized without stopping.

## Verified (2026-10-13 — in progress)

Contracts were committed before implementation. Initial red probes:
```text
cargo test -p bingo-sdk pixel_contract --locked
  dimensions_set_the_picture_estimate ... FAILED (1600 versus 5334)
cargo test -p bingo-store-jsonl the_planned_v2_picture_line --locked
  under_one_kib ... ok; length ... FAILED (432 versus 438)
```
The size assertions were refined by the dated decisions above; encoding
and behavior are unchanged. Further verification follows below.
