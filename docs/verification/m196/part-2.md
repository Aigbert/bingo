# M196 part 2 — verification

- [x] Branch m196-palette on dev `11d46a5a`. Ranking is `bingo_sdk::rank` (prefix 300−len, run
  200−start, letters by spread, ties in order); mod-palette draws its rows through `Sheet::list`
  with `Tally::Counted`, the `@` list with `Tally::Uncounted` (`an_uncounted_list_leaves_the_count_out_and_keeps_its_window ... ok`).
- [x] Keyed captures, pressed through `Run::terminal_event`: `m196_palette_frames_are_pressed_into_the_owner`
  (palette, palette-st, small-palette, ascii-palette), `m196_skill_frames_are_pressed_into_the_owner`
  (skills, skills-review), `m196_mention_frame_is_pressed_into_the_surface ... ok` as full frames;
  skills-sent, mention-inserted and palette-ran by rows/cells, each with its reason in a doc comment.
- [x] git: `git --no-optional-locks status --porcelain=v1 -z` through `bingo_process::tree`, on a task,
  once per opening, 2 s deadline, 1 MiB and 2000-path caps; a stale opening's answer is dropped.
- [x] `unset $(git rev-parse --local-env-vars)`; `rebase -x 'cargo fmt --all -- --check && cargo check
  --workspace --all-targets --locked' dev`: all 13 commits pass. The proof found the palette's three
  deleted files in the sdk pick commit; they moved to the mods commit, the final tree unchanged.
- [x] `cargo fmt --all -- --check`, workspace all-target check, clippy `-- -D warnings`: exit 0.
- [x] `cargo test --workspace --locked` (provider credentials unset): `6630 passed; 0 failed; 0 ignored`,
  TUI `1466 passed`. No wall-clock failure, no retry.
- [x] `scripts/check_discipline.sh`: `discipline ok`. `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- [x] `scripts/budget.sh`:
  ```
  dependencies (unique, normal): 422 (max  423)
  warm cargo check -p bingo-core: 0s (max  20s)
  relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
  target/debug: 11 GB (soft max  5)
  test binaries: 185
  budget ok
  ```
- [x] Windows `--target x86_64-pc-windows-msvc`: bingo-sdk and bingo-mod-palette exit 0.
- [ ] Windows bingo-mods and bingo-surface-tui stop in the aws-lc-sys native build, as in Part 1.
- [x] `python3 docs/verification/m196/drive.py`: `PASS 100x32`, `PASS 80x24` with `/palette/@`, both `CLEAN`.
  New captures: palette, palette-ran, mention, mention-inserted at both sizes.
- Departures: skills-sent's answer arrives whole in the harness (the prototype is mid-reveal);
  palette-ran waits on M199 (branch, `3 changed`) and M200 (agents); mention-inserted's caret sits
  past the space the prototype's caret covers; `/exit` is hidden from the list but runs; command
  hints in the capture world follow the prototype, the shipped crates keep their own.
