# ADR-0067 — The picture is kept once

Status: proposed · 2026-10-13 · Plan: M115 · Amends: ADR-0005 §8,
ADR-0040 §3, ADR-0061 §1, ADR-0062 §1

## Context

A picture is base64 inside the frame that carries it
(`bingo_sdk::Image { media_type, data: String, path }`). The same bytes
are then in the journal line, in the actor's journal copy, in every
`Item` clone (a turn snapshot, a client snapshot, a tree attachment),
in the checkpoint ADR-0066 adds, and on every replay a client asks for.
ADR-0005 §8 already said what comes next: "when a journal grows past
what a replay should read, `ContentPart` gains a blob reference and the
store a `blobs/<hash>` directory — a format change with a migrator".
The `blender_demo` failure (2026-09-16) was that growth: an 8.8 MB
replay through a relay with a 60 s idle cut.

Two smaller facts sit beside it. Every picture costs a flat 1 600
tokens in the estimate (`bingo-sdk/src/tokens.rs`, `IMAGE_TOKENS`),
whatever its size, although `bingo-pictures` knows the pixels it
produced. And ADR-0061 bounds how long a picture stays on the wire, not
how many pictures one request may carry.

No kernel door is opened: blob storage is the store's, the wire is
unchanged (§5).

## Decision

1. **In memory a picture is shared.** `Image.data` becomes
   `Arc<str>` (still base64): cloning an item never copies a picture.
   `Image` gains optional `width` and `height`, set by whoever produced
   the pixels (`bingo_pictures::bounded` and its three doors, ADR-0062
   §1); a picture a wire client sends inline has none (ADR-0040 §3: the
   kernel decodes nothing).
2. **On disk a picture is a blob.** The store writes each picture once
   to `<session>/blobs/<sha256>` (the decoded bytes, hashed by
   `bingo-digest`, ADR-0066; temporary file and rename) before the line
   that refers to it, and the line carries
   `{mediaType, blob, bytes, width?, height?, path?}` instead of `data`.
   The same picture sent twice is one file. Blobs go with the session
   directory and nowhere else.
3. **This is journal version 2,** the first breaking change under
   ADR-0065: a version-1 reader would read the reference as an unknown
   body and lose the item, so it must refuse instead. A version-2 store
   appending to a version-1 journal writes the version line first;
   version-1 lines (inline `data`) read unchanged, so the 1→2 step is
   the identity.
4. **A missing blob is not corruption.** It reads as one text part,
   `[picture missing: <media type> <size>]`, so the session still opens
   and the model is told.
5. **The wire is unchanged.** Live frames and `events_since` carry the
   picture inline, as today; the store resolves references on replay.
   A client that wants pictures by reference already has M107's bounded
   transport, which pins and pages large parts.
6. **The estimate reads the pixels.** A picture with `width` and
   `height` costs `ceil(width × height / 750)` tokens; one without
   keeps the flat `IMAGE_TOKENS`.
7. **A request carries a bounded weight of pictures.** After ADR-0061's
   age rule, pictures are kept newest first until
   `budget::REQUEST_PICTURE_BYTES = 4 MB` of decoded bytes is spent;
   every older one is projected to ADR-0061's elision note. A projection,
   not a record; the threshold lives beside `IMAGE_ROUNDS_KEPT`.

## Consequences

- A session with many screenshots stops growing its journal by the
  picture each time it is quoted; resume and checkpoint read
  references, and the blobs only for the window ADR-0066 keeps.
- A version-1 binary refuses a journal a version-2 binary has appended
  to, with the "newer bingo" message — the intended failure.
- `Image.data`'s type change touches every provider's request builder;
  they read it as `&str`, so the change is mechanical.
- The estimate gets closer for small pictures and stops undercounting a
  2000 px one (≈5 300 tokens, not 1 600), so compaction triggers earlier
  for picture-heavy sessions.
- The request picture budget can elide a picture the model has seen
  fewer than three times; the note still names its file.

Refs: ADR-0005, ADR-0040, ADR-0052, ADR-0061, ADR-0062, ADR-0065,
ADR-0066; Plans: M107, M115; audit-2026-10 §1
