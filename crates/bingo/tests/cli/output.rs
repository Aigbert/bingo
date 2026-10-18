//! Drain both pipes while a child runs; inherited pipes cannot outlive the deadline.
use std::io::{self, Read};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use super::*;

pub(super) struct Pipe {
    bytes: Arc<Mutex<Vec<u8>>>,
    reader: JoinHandle<io::Result<()>>,
}

impl Pipe {
    pub(super) fn start<R: Read + Send + 'static>(pipe: Option<R>) -> Self {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let captured = bytes.clone();
        let reader = std::thread::spawn(move || {
            if let Some(pipe) = pipe {
                drain(pipe, &captured)?;
            }
            Ok(())
        });
        Self { bytes, reader }
    }

    pub(super) fn collect(self) -> Vec<u8> {
        // Let a closing pipe finish draining, but a grandchild may still own it.
        let _ = wait_within(Duration::from_secs(1), async {
            while !self.reader.is_finished() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
        if self.reader.is_finished() {
            self.reader
                .join()
                .expect("the reader thread")
                .expect("read the pipe");
        }
        self.bytes.lock().unwrap().clone()
    }
}

fn drain(mut pipe: impl Read, bytes: &Mutex<Vec<u8>>) -> io::Result<()> {
    let mut buffer = [0; 8192];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(length) => bytes.lock().unwrap().extend_from_slice(&buffer[..length]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn wait(
    child: &mut std::process::Child,
    stdout: &Pipe,
    stderr: &Pipe,
    limit: Duration,
) -> bool {
    wait_within(limit, async {
        loop {
            if child.try_wait().expect("wait").is_some()
                && stdout.reader.is_finished()
                && stderr.reader.is_finished()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .is_ok()
}

#[test]
fn a_run_drains_more_than_a_pipe_capacity_from_both_streams() {
    let mut command = fixture_command();
    let output = run_before(&mut command, PATIENCE, "the output fixture blocked");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(output.stdout.ends_with(&vec![b'x'; 256 * 1024]));
    assert_eq!(output.stderr, vec![b'y'; 256 * 1024]);
}

fn fixture_command() -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "output::large_output_fixture", "--nocapture"])
        .env("BINGO_TEST_LARGE_OUTPUT", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn large_output_fixture() {
    if std::env::var_os("BINGO_TEST_LARGE_OUTPUT").is_none() {
        return;
    }
    std::io::stdout()
        .write_all(&vec![b'x'; 256 * 1024])
        .unwrap();
    std::io::stderr()
        .write_all(&vec![b'y'; 256 * 1024])
        .unwrap();
    std::process::exit(0);
}

#[test]
fn an_open_pipe_returns_captured_bytes_without_waiting_for_eof() {
    let (entered, blocked) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let reader = b"captured before timeout"
        .as_slice()
        .chain(HeldOpen { entered, released });
    let pipe = Pipe::start(Some(reader));
    blocked.recv_timeout(PATIENCE).unwrap();
    let (done, result) = std::sync::mpsc::channel();
    let collecting = std::thread::spawn(move || done.send(pipe.collect()).unwrap());
    let captured = result.recv_timeout(PATIENCE);
    // Release the fixture even if collecting incorrectly waited for EOF.
    release.send(()).unwrap();
    collecting.join().unwrap();
    assert_eq!(captured.unwrap(), b"captured before timeout");
}

struct HeldOpen {
    entered: std::sync::mpsc::Sender<()>,
    released: std::sync::mpsc::Receiver<()>,
}

impl Read for HeldOpen {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        self.entered.send(()).unwrap();
        self.released.recv().unwrap();
        Ok(0)
    }
}
