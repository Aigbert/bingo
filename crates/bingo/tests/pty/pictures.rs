//! Pictures through a real pty (M46–M60): what the terminal at the other end
//! answers decides whether a picture goes out as cells and bytes or stays the
//! chip that names it, and only a terminal can answer.

use super::*;

/// Ask for the picture and open the turn that read it: a finished turn folds
/// to one line, and what a step brought back shows under it opened (M185).
fn look(terminal: &mut Terminal) {
    terminal.send(b"look at it\r");
    open_the_turn(terminal);
}

/// Open the finished turn whose answer is the picture's. What is waited for
/// next is the picture or its chip: a tall picture scrolls the turn's own
/// line out of sight.
fn open_the_turn(terminal: &mut Terminal) {
    terminal.wait_for("That is the picture.");
    terminal.send(&[0x0f]);
}

/// A picture in the transcript on a terminal that says it draws pictures:
/// the bytes of it go out once, as a virtual placement, and the chip that
/// stands in for one everywhere else is not drawn (design §5, M46).
#[test]
fn a_terminal_that_answers_the_graphics_probe_is_sent_the_picture() {
    let file = bingo_pictures::testing::png_bytes(1200, 900);
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::Pictures);
    std::fs::write(terminal.home.path().join("shot.png"), &file).unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    // The cells are on the screen a frame before the pixels are: fitting a
    // picture to them is a decode, and the run does it off the loop (M61).
    let written = terminal.wait_transmission();
    assert_eq!(
        count(&written, b"\x1b_Ga=T,f=100"),
        1,
        "the picture went out once, whole, and only once"
    );
    assert!(
        contains(&written, b"U=1"),
        "as a virtual placement the cells stand in for"
    );
    // M48 brick 2: the bytes are cut to the cells they will cover, so a
    // picture far bigger than its block costs the block and not itself.
    let sent = transmitted(&written);
    let size = bingo_pictures::png_size(&sent).expect("a PNG went out");
    assert!(
        size.0 <= 1200 && size.1 < 900,
        "the block's pixels, not the file's: {size:?}"
    );
    assert!(
        sent.len() * 4 < file.len(),
        "{} of {} bytes went out",
        sent.len(),
        file.len()
    );
    assert!(
        !terminal.screen().contains("[image:"),
        "and no chip was drawn:\n{}",
        terminal.screen()
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// A picture the answer named in its own words, on a terminal that draws
/// pictures (M51): the chip stays as the line it hangs from, the file is read
/// in between frames, and the bytes go out once.
#[test]
fn a_picture_an_answer_named_in_markdown_is_read_in_and_sent() {
    let mut terminal = Terminal::opened(&[], NAMES_A_PICTURE, Answers::Pictures);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(300, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    terminal.send(b"show me the shot\r");
    terminal.wait_for("[image: the shot]");
    // The read happens between frames, so the bytes come after the chip.
    let written = terminal.wait_transmission();
    assert_eq!(
        count(&written, b"\x1b_Ga=T,f=100"),
        1,
        "read in once and sent once"
    );
    assert!(contains(&written, b"U=1"), "as a virtual placement");
    let sent = transmitted(&written);
    assert!(
        bingo_pictures::png_size(&sent).is_some(),
        "and what went out is a PNG"
    );
    let screen = terminal.screen();
    assert!(
        screen.contains("[image: the shot]"),
        "the chip stays: it is the line the picture hangs from\n{screen}"
    );
    assert!(
        screen.contains("That is all."),
        "and the words after it are still there\n{screen}"
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// The PNG of the one `a=T` transmission in `written`, decoded out of the APC
/// sequence that carried it.
fn transmitted(written: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let at = written
        .windows(6)
        .position(|w| w == b"\x1b_Ga=T")
        .expect("a transmission");
    let body = &written[at..];
    let keys = body.iter().position(|b| *b == b';').expect("a payload") + 1;
    let end = body
        .windows(2)
        .position(|w| w == b"\x1b\\")
        .expect("a terminator");
    base64::engine::general_purpose::STANDARD
        .decode(&body[keys..end])
        .expect("base64")
}

/// M48 brick 1: a terminal that answers the graphics query and is not known
/// to draw a placeholder cell gets the chip. It would otherwise paint tofu
/// where the picture goes and leave a stray copy of it at the cursor
/// (wezterm#986, bugs.kde.org 523718).
#[test]
fn a_terminal_that_says_ok_and_draws_no_placeholder_gets_the_chip() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::Tofu);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(100, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    terminal.wait_for("[image: image/png]");
    assert!(
        !contains(&terminal.written(), b"\x1b_Ga=T"),
        "nothing was transmitted to a terminal that would not draw it"
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// The same picture on a terminal that answers only DA1: the chip, and not
/// one byte of graphics protocol.
#[test]
fn a_terminal_that_answers_only_da1_gets_the_chip() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::Da1Only);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(100, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    terminal.wait_for("[image: image/png]");
    assert!(
        !contains(&terminal.written(), b"\x1b_Ga=T"),
        "nothing was transmitted to a terminal that cannot draw it"
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// M49: inside tmux the probe and the picture both travel in the passthrough
/// envelope, and the outer terminal — the one that answers out of it — is
/// what decides whether there is a picture at all.
#[test]
fn a_picture_under_tmux_travels_in_the_passthrough_envelope() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::ThroughTmux);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(1200, 900),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    terminal.wait_written(WRAPPED_TRANSMIT);
    let written = terminal.written();
    assert!(
        contains(&written, WRAPPED_QUERY),
        "the question went to the terminal behind tmux"
    );
    assert_eq!(
        count(&written, WRAPPED_TRANSMIT),
        1,
        "and so did the picture, once, in an envelope of its own"
    );
    assert!(
        !terminal.screen().contains("[image:"),
        "and no chip was drawn:\n{}",
        terminal.screen()
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// The passthrough off: tmux answers for itself, nothing behind it answers at
/// all, and what a person gets is the chip — not a picture-shaped hole.
#[test]
fn a_tmux_that_carries_nothing_through_gets_the_chip() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::TmuxAlone);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(100, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    terminal.wait_for("[image: image/png]");
    let written = terminal.written();
    assert!(
        contains(&written, WRAPPED_QUERY),
        "it was asked through the envelope all the same"
    );
    assert!(
        !contains(&written, b"\x1b_Ga=T"),
        "and nothing was transmitted to a terminal that never answered"
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// M60 bricks 1, 2 and 4: the outer terminal finishes answering after the
/// probe's window has run out. Its reply lands in crossterm's key stream —
/// `alt+P`, `>`, `|`, the name, `alt+\` — where before it was typed into the
/// composer and made the box grow a row. Nothing of it reaches the composer
/// now, and the answer it carries turns the pictures on for the next frame.
#[test]
fn an_answer_that_lands_after_the_probe_is_eaten_and_still_counted() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::ThroughTmuxLate);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(1200, 900),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    terminal.wait_late();
    // A keystroke behind the late answer, and the screen that shows it: the
    // pty keeps its order, so a composer holding this and nothing else is a
    // composer the reply never reached. The box is where the bug showed —
    // `> Gi=31;OK>|ghostty 1.3.1`, and a second row of it once the words
    // outgrew the width, which is the layout jump (M60 Verified).
    terminal.send(b"look at it");
    terminal.wait_for("look at it");
    let composer = terminal.screen();
    for typed in ["Gi=31", ">|ghostty", "OK>"] {
        assert!(
            !composer.contains(typed),
            "{typed:?} was typed into the composer:\n{composer}"
        );
    }
    terminal.send(b"\r");
    open_the_turn(&mut terminal);
    terminal.wait_written(WRAPPED_TRANSMIT);
    let written = terminal.written();
    assert_eq!(
        count(&written, WRAPPED_TRANSMIT),
        1,
        "the picture the late answer paid for went out, once, in an envelope"
    );
    assert!(
        !terminal.screen().contains("[image:"),
        "and no chip was drawn:\n{}",
        terminal.screen()
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// M60 bricks 3 and 4: tmux says `allow-passthrough` is off, so the question
/// is not asked at all. Nothing wrapped is written, no window is spent waiting
/// for an answer that was promised not to come, and the chip is what a picture
/// draws as.
#[test]
fn a_passthrough_tmux_says_is_off_is_never_asked_through() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::TmuxPassthroughOff);
    std::fs::write(
        terminal.home.path().join("shot.png"),
        bingo_pictures::testing::png_bytes(100, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    terminal.wait_for("[image: image/png]");
    let written = terminal.written();
    assert!(
        !contains(&written, WRAPPED_QUERY),
        "no envelope was sent into a passthrough that drops it"
    );
    assert!(
        !contains(&written, GRAPHICS_QUERY),
        "and the question was not asked bare either"
    );
    // The wait that is not spent is the question that is not asked: there is
    // no window to time, because nothing was ever waited for. A wall-clock
    // bound here would pin this machine instead (AGENTS.md).
    assert!(
        !contains(&written, b"\x1b_Ga=T"),
        "and nothing was transmitted to a terminal that was never reached"
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

// ---- Ghostty, and Ghostty inside a herdr pane ----------------------------

/// The kitty placeholder cell: one of them stands on every cell a picture
/// covers, and a terminal that draws pictures draws it as the picture.
const PLACEHOLDER: &[u8] = "\u{10EEEE}".as_bytes();

/// An answer that names a picture under the person's home by `~`, with words
/// outside ASCII in its alternative text — the way an answer about a desktop
/// file is written.
const NAMES_A_PICTURE_AT_HOME: &str = r#"{"responses":[
  {"steps":[{"text":"![桌面上的 lt.png](~/x/lt.png)\n\nThat is all."}]}]}"#;

/// Whether the picture went out as a virtual placement and its cells reached
/// the screen: what a terminal that draws pictures is sent.
fn assert_drawn(terminal: &Terminal) {
    let written = terminal.wait_transmission();
    assert_eq!(count(&written, b"\x1b_Ga=T,f=100"), 1, "sent once");
    assert!(contains(&written, b"U=1"), "as a virtual placement");
    assert!(
        contains(&written, PLACEHOLDER),
        "and the cells it stands on were written"
    );
}

/// Ghostty 1.3.1 says its name, its cell and `OK`: a picture an answer named
/// under `~` is read from the home and drawn in placeholder cells.
#[test]
fn ghostty_draws_a_picture_an_answer_named_under_home() {
    let mut terminal = Terminal::opened(&[], NAMES_A_PICTURE_AT_HOME, Answers::Ghostty);
    let shots = terminal.home().join("x");
    std::fs::create_dir_all(&shots).unwrap();
    std::fs::write(
        shots.join("lt.png"),
        bingo_pictures::testing::png_bytes(300, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    terminal.send(b"show me the desktop file\r");
    terminal.wait_for("That is all.");
    assert_drawn(&terminal);
    assert!(
        !terminal.screen().contains("not found"),
        "the path under ~ was found\n{}",
        terminal.screen()
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// A herdr pane with `kitty_graphics` on says `OK` and a bare `libghostty`,
/// and never its cell — but its window has pixels, and the cell is read from
/// them.
#[test]
fn a_herdr_pane_that_draws_pictures_is_sent_the_picture() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::HerdrOn);
    std::fs::write(
        terminal.home().join("shot.png"),
        bingo_pictures::testing::png_bytes(300, 200),
    )
    .unwrap();
    terminal.wait_for("/ commands");
    look(&mut terminal);
    assert_drawn(&terminal);
    assert!(
        !terminal.screen().contains("[image:"),
        "and no chip was drawn\n{}",
        terminal.screen()
    );
    terminal.send(&[0x04]);
    terminal.leave();
}

/// The same pane with the setting off answers the same and has a window of
/// no pixels: the chip, nothing transmitted, and one line naming the setting
/// that would turn the pictures on.
#[test]
fn a_herdr_pane_with_pictures_off_gets_the_chip_and_the_setting() {
    let mut terminal = Terminal::opened(&[], READS_A_PICTURE, Answers::HerdrOff);
    std::fs::write(
        terminal.home().join("shot.png"),
        bingo_pictures::testing::png_bytes(100, 200),
    )
    .unwrap();
    terminal.wait_for("herdr: pictures need 0.9");
    look(&mut terminal);
    terminal.wait_for("[image: image/png]");
    assert!(
        !contains(&terminal.written(), b"\x1b_Ga=T"),
        "nothing was transmitted to a pane that would leak the cells"
    );
    terminal.send(&[0x04]);
    terminal.leave();
}
