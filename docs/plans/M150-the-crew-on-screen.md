# M150 — The crew on screen

## Goal

Implement [tui-identity §4 C1–C4](../design/tui-identity-2026-10.md):
name-derived agent faces, the active crew in the status middle, and rooms
read as chats. Identity uses hue; lifecycle keeps its words and pulse.
No new dependency, SDK type, wire shape or schema change.

## Bricks, in build order

1. **Contract first:** `fixtures/faces.json`, twenty names including UTF-8,
   empty and case-distinct names. FNV-1a 32-bit over exact UTF-8 bytes;
   `h % 6` selects `◆ ● ▲ ■ ◗ ✦` (ASCII `@ # % & + =`),
   `(h >> 8) % 5` selects lime, teal, violet, orchid, pink.
2. **Failing screen contracts:** fixture/ASCII equality; all five hues
   ≥4.5:1 against their palette's raised ground and ≥36° from the four
   state accents; child faces and lifecycle words; roster faces with `~`;
   wide/narrow crew, debt, notice handover; grouped room authors, reader
   marker, members, known mentions. Seven tests fail before implementation.
3. **Faces and rows:** a pure hash in `face.rs`, hues in `theme.rs`;
   `child_row`, roster `dot` and room `post` derive the face. Remove the
   superseded tree `bullet_style`. ANSI/plain use text; live sigils pulse
   in weight. Keep M135's `working`, `asking`, `idle · completed` words.
4. **Crew:** working/asking descendants of the viewed session, in tree
   order; names at ≥100 columns, sigils below; `+N` for omitted entries.
   Read `owes` through `seats::seat`; clicking opens `ctrl+g`.
   Priority: armed exit, notice, placed Status, crew, existing idle hint.
5. **Mentions:** known `@agent` and `#room` take identity and mode colours;
   unknown names stay plain. Markdown code and links keep their treatment.
   Names come from the folded tree, never a plugin query.
6. **Chat:** consecutive authors share one heading; calls on the holder
   (`parent`, including `@all`) end `→ you` (`-> you` in ASCII). Members
   take one clipped dim row below the first post. Cache revisions include
   known names, room sequence and the child's pulse; layout owns hit areas.
7. **Records:** update tui.md §§3, 4, 10 and tick identity C1–C4.

## Files

- TUI `src/{face,theme,transcript,tree,roster,status,markdown,crew}.rs`,
  `transcript/said.rs`, `markdown/mentions.rs`, `fixtures/faces.json`.
- TUI wiring: `src/{lib,blocks,seats,pointer,layout,view}.rs`,
  `layout/deadline.rs`; TUI test modules/fixtures and their snapshots.
- `scripts/tui-smoke.sh`; `docs/design/{tui,tui-identity-2026-10}.md`.
- Changed snapshots: 28, in child-row, room, roster/switcher and status
  families. Composer/status geometry stays fixed; chat membership is content.

## Exit criteria

- [x] All seven original contracts fail first and pass after implementation.
- [x] No `N running` drawing remains in status.rs.
- [x] Fake-provider tmux drive: parent, two agents and a room; both sizes
      (120×40, 80×24), dark/light/NO_COLOR/ASCII; strip, one agent ending,
      asking pulse, list, grouped posts, mentions and members.
- [x] Snapshot families reviewed; layout tests pin unchanged furniture rows.
- [x] `scripts/tui-smoke.sh` passes.
- [x] Native protocol gates pass; Windows cross-check attempted under step 7.

## Non-goals

Chosen faces/personas, print/channel faces, GUI implementation, changed room
records or delivery (ADR-0034), or identity colour outside sigils/names.
M137's plugin-host, plugin-rpc, plugin manifests and schemas are untouched.

## Risks

- Re-verified at `9150697a` on 2026-10-14, after M108–M136 and M147–M149,
  in commit `4d802d67`. ADR-0034, 0070/0071, 0077 and 0081 §§1–6 hold.
  `Lifecycle::of_state` owns the words. `Tree::sessions` is root/id order;
  `Scope::Under` resolves descendants. Collab readers own owed/members data.
- The room view is `transcript/said.rs`; `Place::previous` supplies grouping.
  Names and membership must invalidate cached drawings even without new text.
- Faces can collide; names remain available in the list at narrow widths.
- Dark pink becomes `#f08cc0` (proposed pink was only 35.53° from bad);
  light teal becomes `#1d7566` (proposed teal was 4.25:1 on raised).
  State accents are unchanged. All five hues satisfy both palette checks.
- Notice priority preserves its four-second hold and two 66 ms fades.
- Windows: `aws-lc-sys` lacks `stdlib.h`/`windows.h` on this Mac. Per protocol
  step 7, the TUI cross-check is left to Windows CI; no toolchain installed.

- Drive selectors confused room membership and transcript prompts with selected roster names; the event regression preserves #design, as tui.md §3 designs.

## Verified (2026-10-14)

```text
$ cargo test -p bingo-surface-tui --lib crew_contract --locked
test crew_contract::crew_hues_are_readable_and_apart_from_state ... FAILED
test crew_contract::faces_match_the_cross_surface_fixture ... FAILED
test crew_contract::strip_names_the_work_and_yields_to_notices ... FAILED
test crew_contract::roster_keeps_a_face_beside_the_live_ear ... FAILED
test crew_contract::child_row_has_a_face_and_keeps_lifecycle_words ... FAILED
test crew_contract::room_groups_authors_marks_the_reader_and_lists_members ... FAILED
test crew_contract::room_mentions_wear_the_known_names_hues ... FAILED
test result: FAILED. 0 passed; 7 failed; 0 ignored
$ cargo fmt --all -- --check
exit=0
$ cargo check --workspace --all-targets --locked
exit=0
$ cargo clippy --workspace --all-targets --locked -- -D warnings
exit=0
$ cargo test --workspace --locked
test crew_contract::a_child_permission_and_room_posts_keep_the_selected_room_in_front ... ok
test crew_contract::a_room_query_also_matches_its_members_and_keeps_the_cursor ... ok
exit=0; 5171 passed; 0 failed; 4 ignored
$ scripts/check_discipline.sh
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 355 (max 355)
warm cargo check -p bingo-core: 0s (max 20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 5 GB (soft max 5)
test binaries: 102
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p bingo-surface-tui --all-targets --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
fatal error: 'stdlib.h' file not found; fatal error: 'windows.h' file not found
$ cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
test result: ok. 1 passed; 0 failed; 0 ignored
$ cargo test -p bingo-provider-acp --test bridge --locked token -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored
$ cargo build -p bingo --locked
exit=0
$ scripts/tui-smoke.sh
tui-smoke ok
$ rg 'running' crates/bingo-surface-tui/src/status.rs
611:        assert!(!drawn.contains("running"), "{drawn}");
681:        // Nothing published, nothing running, nothing to say, and something
$ python3 /tmp/m150-drive.py
AssertionError: missing First review post:
$ python3 /tmp/m150-drive.py
AssertionError: missing First review post:
$ python3 /tmp/m150-drive.py
AssertionError: exact selection missing:
$ python3 /tmp/m150-drive.py
PASS dark 120x40: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS dark 80x24: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS light 120x40: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS light 80x24: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS plain 120x40: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS plain 80x24: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS ascii 120x40: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
PASS ascii 80x24: child faces, two-agent strip, asking pulse, list, grouped room posts, mentions, members, reader marker
```
