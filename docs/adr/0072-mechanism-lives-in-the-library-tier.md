# ADR-0072 — Mechanism lives in the library tier

Status: proposed · 2026-10-13 · Plans: M109 (claim), M110 (shell), M127, M128, M129 · Supersedes: ADR-0056 §5

## Context

No plugin imports another (ADR-0001), so every mechanism two plugins
needed was copied; ADR-0056 §5 made that the rule for the providers.
ADR-0012 §1 and ADR-0042 §2 have since opened a library tier: it
registers nothing, depends on the sdk and other libraries, and any
plugin may depend on it. `bingo-loopback` exists because two callers
needed one socket. The copies audit-2026-10 §5 counted:

- **HTTP.** Both providers carry the same `metered.rs`, `sse.rs` and
  `stream.rs`. Each `http()` falls back to `reqwest::Client::new()`,
  which panics exactly where the builder failed. Three clients have no
  bound at all.
- **Process trees.** Three crates make a group or job object. MCP stdio
  servers and plugins do not, so an `npx` server's grandchildren likely
  outlive it (inferred; M128 proves it first).
- **JSON-RPC.** Three pending maps: surface-rpc, plugin-rpc ("deliberate",
  ADR-0015) and ACP. Two readers are unbounded `BufReader::lines()`.
- **Claims.** Four lock styles. Two are pid files whose existence is the
  lock, so a killed holder leaves them behind.
- **Shelves.** Schedule and experience share one file-per-id store and
  `id.rs`. `diff.rs` exists three times.

M109 has already made `bingo-claim` and M110 `bingo-process`
(ADR-0063). This ADR is the rule they followed, and its full scope.

## Decision

1. **Mechanism two plugins share is a library crate, never a copy.** One
   noun per crate, `tier = "library"`. A copy stays only where the tier
   cannot reach (the kernel), with a comment naming this ADR. ADR-0056's
   numbers (20 s connect, 300 s idle, 64 KiB frame) are defined once.
2. **`bingo-http`: a bounded client and a streamed response.**
   - `client(Bounds) -> Result<Client, HttpError>`: a builder that fails
     is an error, never an unbounded client.
   - `metered`, `sse`, and `stream`, which turns chunks into a
     `ModelStream` through a `Decode` each provider implements.
   - `round_trip`, which returns the response and its clock.

   Each provider still classifies its own refusals. The providers' twin
   `key.rs` is credential lookup, not HTTP: it moves to
   `bingo-auth-oauth`, which it already names as its home.
3. **`bingo-process`: a program and the tree it starts.** It holds
   `shell` (ADR-0063) and `spawn`. `spawn` returns a `Tree`: a process
   group on unix, a job object on Windows, killed on drop. A tree holds
   its pipes, sends stderr to a log or nowhere, and offers
   `terminate(grace)`. Every child started for a tool, hook, MCP server,
   plugin or ACP agent is a `Tree`. MCP hands rmcp the pipes and rmcp
   spawns nothing. The gateway's detached start is a process meant to
   outlive bingo, a different noun, and stays as it is.
4. **`bingo-jsonrpc`: JSON-RPC 2.0 over lines.**
   - `codec`: the messages, the standard codes, and `lines(max)`. Every
     reader is bounded; an over-long line is a named error that ends the
     connection.
   - `Peer`: the one pending map. A dropped request removes its entry.
     A wire cancel stays the protocol's own message (`tool/cancel`,
     `session/cancel`). The library never invents one.

   Vocabularies stay with their protocols: surface-rpc's methods and
   M107 budgets, plugin-rpc's `TRANSPORT_ERROR`, ACP's schema crate.
   MCP stays on rmcp, which implements that protocol.
5. **`bingo-claim`: one process holds a name.** A claim is an OS lock on
   a permanent file, so a kill releases it. `Claim::take` writes a
   marker that tells a claim from a legacy file. Below it, `lock(path)`
   serves a lock with no marker: the store's sidecars (ADR-0005).
   Schedule and channels move in M109; the gateway and the store in
   M129.
6. **`bingo-shelf`: files bingo writes for a person to keep.**
   `Shelf<C: Codec>` gives `load` (sorted, unreadable listed with why),
   `save` (temp file plus rename), `delete` (missing is done), `mint`
   and `resolve`. The codec owns the extension and the encoding: JSON
   for schedules, Markdown frontmatter for experience.
   `diff::unified` shows a write before it lands, so it lives here too.
7. **Not every helper is a crate.** A library needs a noun with two
   users today; one user means a module, and a third copy is the
   signal. The kernel takes no library edge for this, so core's atomic
   writers stay. Changing that would take its own ADR.

## Consequences

- Five members, each raising `max_dependencies` by one with a comment.
  None brings an external crate the tree lacks, and process-wrap 9
  leaves the shipped tree.
- A fix to the stream, spawner, reader or claim reaches every user. MCP
  servers and plugins gain trees; plugin-rpc and ACP gain bounded
  readers.
- Tests move with their code and pass against the library before the old
  copies go. ADR-0015's deliberate copy ends; the plugin wire stays its
  own.
- The library tier grows from four crates to nine. `check_discipline.sh`
  already enforces the direction.

## Supersedes

ADR-0056 §5, "Two copies, one record". Its §1–4 stand.

Refs: ADR-0001, ADR-0005, ADR-0012, ADR-0015, ADR-0042, ADR-0056,
ADR-0063; Plans: M109, M110, M127, M128, M129; audit-2026-10 §5
