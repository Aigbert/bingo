//! Terminal probe I/O, before the alternate screen is taken.

use crate::graphics::{Cell, Heard, Passthrough, Transport, probe};

/// Put the three queries on the terminal and read until DA1 comes back.
///
/// The terminal is opened directly rather than through stdout, so a run whose
/// output is redirected still asks the terminal a person is looking at, and
/// the handle is non-blocking, so a terminal that answers nothing costs the
/// timeout and not the run: a blocking read of a tty has no deadline, and a
/// thread abandoned in one would hold a lock the next frame wants.
#[cfg(unix)]
pub(super) fn exchange(transport: Transport, passthrough: Passthrough) -> Vec<u8> {
    use std::io::Write;

    // tmux says it drops the envelope, so none is sent and nothing is waited
    // for: the question would reach nobody and the window would be spent on a
    // silence that was promised in advance (M60 brick 3).
    if passthrough == Passthrough::Off {
        return Vec::new();
    }
    let Ok(mut tty) = tty() else {
        return Vec::new();
    };
    // Raw mode, or the answer waits for a newline that will never be typed.
    if crossterm::terminal::enable_raw_mode().is_err() {
        return Vec::new();
    }
    let query = probe::query(transport);
    let asked = tty.write_all(&query).and_then(|()| tty.flush());
    let answer = match asked {
        Ok(()) => listen(&mut tty, window(transport, passthrough)),
        Err(_) => Vec::new(),
    };
    let _ = crossterm::terminal::disable_raw_mode();
    answer
}

/// The answer, with a cell read off the window when the terminal said none.
/// Only bare: under tmux the pane's window is tmux's own guess at the pixels,
/// and the terminal behind it is the one whose cells a picture is drawn in.
pub(super) fn measured(mut heard: Heard) -> Heard {
    if heard.transport == Transport::Bare && heard.probe.cell.is_none() {
        heard.probe.cell = window_cell();
        heard.measured = heard.probe.cell.is_some();
    }
    heard
}

/// The cell the terminal's window says, from its size in pixels and in cells
/// ([`probe::window_cell`]).
#[cfg(unix)]
fn window_cell() -> Option<Cell> {
    let window = crossterm::terminal::window_size().ok()?;
    probe::window_cell(window.columns, window.rows, window.width, window.height)
}

/// No Windows console is asked anything ([`exchange`]), and none has a window
/// in pixels to read (crossterm answers `Unsupported`).
#[cfg(not(unix))]
fn window_cell() -> Option<Cell> {
    None
}

/// How long the answers are given. Under a tmux that says it carries them
/// they have three legs to travel rather than one, so they get three times as
/// long ([`crate::theme::PROBE_THROUGH`]); whatever still arrives after that
/// is heard out of the key stream instead ([`late`]).
#[cfg(unix)]
fn window(transport: Transport, passthrough: Passthrough) -> std::time::Duration {
    match transport {
        Transport::Tmux if passthrough.carries() => crate::theme::PROBE_THROUGH,
        _ => crate::theme::PROBE,
    }
}

/// No Windows console host speaks the kitty graphics protocol, and a console
/// that will never answer would cost every start-up the whole timeout. The
/// question is not asked there — design §5's chip is what is drawn, which is
/// what an unanswered question comes to anyway.
#[cfg(not(unix))]
pub(super) fn exchange(_transport: Transport, _passthrough: Passthrough) -> Vec<u8> {
    Vec::new()
}

/// The terminal itself, opened so that a read answers at once whether or not
/// anything has arrived (`O_NONBLOCK`). A process with no controlling
/// terminal has none to open, and draws no pictures.
#[cfg(unix)]
fn tty() -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
}

/// Read until the terminal has answered or the clock runs out. Nothing here
/// blocks: an empty read is a wait, and the waiting is bounded.
#[cfg(unix)]
fn listen(tty: &mut std::fs::File, window: std::time::Duration) -> Vec<u8> {
    use std::io::Read;

    let deadline = std::time::Instant::now() + window;
    let mut answer = Vec::new();
    let mut buffer = [0u8; 256];
    while !probe::answered(&answer) && answer.len() < MOST {
        match tty.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => answer.extend_from_slice(&buffer[..read]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(BETWEEN);
            }
            Err(_) => break,
        }
    }
    answer
}

/// How long to wait between two looks at a terminal that has not answered
/// yet. Short enough that a start-up spends no visible time on it, long
/// enough that the wait is not a spin.
#[cfg(unix)]
const BETWEEN: std::time::Duration = std::time::Duration::from_millis(2);

/// The most an answer may be. A terminal that will not stop talking is not
/// answering the question that was asked.
#[cfg(unix)]
const MOST: usize = 4096;
