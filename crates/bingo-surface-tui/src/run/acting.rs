//! Acting scenes enter through the same terminal, frame and reply boundaries as a run.
use super::*;
use crate::doubles::{Recorder, TestSession};
use crate::test_support::{
    self as fixture,
    ember::{CLOCK_ORIGIN_MS, Capture, answers, story},
};
use bingo_sdk::ItemBody;
use serde_json::json;
use std::sync::{Arc, Mutex};

mod checks;
#[cfg(test)]
mod proofs;
mod setup;
mod timeline;

const QUEUED: &str = "做完以后更新一下 CHANGELOG";

struct Scene {
    run: Run,
    session: Arc<TestSession>,
    host: bingo_sdk::testing::TestHost,
    clock: Arc<Mutex<Now>>,
    replies: mpsc::Receiver<Reply>,
    screen: Recorder,
    _home: tempfile::TempDir,
    timeline: Vec<(i64, timeline::Answer)>,
    sent: usize,
    interrupted: usize,
    revision: u64,
    name: String,
    turns: Vec<serde_json::Value>,
}

impl Scene {
    fn emit(&mut self, event: Event) {
        let mut frame = fixture::frame(self.run.session.tree.root().seq.0 + 1, event);
        frame.ts = self.run.now().wall;
        self.run.frame(&frame, &mut self.screen).unwrap();
    }

    fn emit_all(&mut self, events: Vec<Event>) {
        for event in events {
            self.emit(event);
        }
    }

    fn set_time(&self, at: i64) {
        let mut now = self.clock.lock().unwrap();
        let change = at + CLOCK_ORIGIN_MS - now.wall.as_millisecond();
        now.instant = if change >= 0 {
            now.instant + Duration::from_millis(change as u64)
        } else {
            now.instant - Duration::from_millis((-change) as u64)
        };
        now.wall = jiff::Timestamp::from_millisecond(CLOCK_ORIGIN_MS + at).unwrap();
    }

    async fn key(&mut self, key: crossterm::event::KeyEvent) {
        let pending = !self
            .run
            .ui
            .pictures
            .carried(self.run.ui.composer.text())
            .is_empty()
            && key.code == crossterm::event::KeyCode::Enter;
        let withdrawing = !self.run.session.tree.viewed().queue.is_empty()
            && key.code == crossterm::event::KeyCode::Esc;
        self.run.terminal_event(Term::Key(key));
        if pending || withdrawing {
            let reply = tokio::time::timeout(Duration::from_secs(10), self.replies.recv())
                .await
                .unwrap()
                .unwrap();
            if let Reply::Stopped(_, ref taken) = reply {
                for (intent, input) in taken {
                    if input.is_ok() {
                        self.revision += 1;
                        self.emit(answers::removed(
                            self.run.session.tree.viewed(),
                            intent,
                            self.revision,
                        ));
                    }
                }
            }
            self.run.reply(reply, &mut None);
        }
        timeline::submitted(self);
        if self.session.interrupts() > self.interrupted {
            self.interrupted = self.session.interrupts();
            self.emit_all(answers::stopped(
                self.run.session.tree.viewed(),
                self.run.now().wall,
            ));
        }
    }

    async fn text(&mut self, text: &str, paced: bool) {
        for ch in text.chars() {
            self.key(fixture::typed(ch)).await;
            if paced {
                timeline::advance(self, self.elapsed() + 60).await;
            }
        }
    }

    fn elapsed(&self) -> i64 {
        self.run.now().wall.as_millisecond() - CLOCK_ORIGIN_MS
    }
}

pub(crate) async fn draw(capture: &Capture) {
    let mut scene = setup::new(capture);
    setup::setup(&mut scene).await;
    for step in &capture.entry.after {
        if let Some(wait) = step.as_i64() {
            let to = scene.elapsed() + wait;
            timeline::advance(&mut scene, to).await;
        } else if let Some(text) = step.as_array() {
            scene.text(text[0].as_str().unwrap(), true).await;
        } else if let Some(key) = step.as_str().filter(|s| !s.starts_with("shot")) {
            scene.key(timeline::key(key)).await;
            let to = scene.elapsed() + 60;
            timeline::advance(&mut scene, to).await;
        }
    }
    timeline::advance(&mut scene, capture.entry.moment_ms).await;
    checks::check(&scene);
    let size = capture.size();
    let now = scene.run.now();
    let backend = fixture::drawn(
        size.width,
        size.height,
        &scene.run.session.tree,
        &mut scene.run.ui,
        now,
    );
    crate::test_support::ember::assert_band(
        capture,
        crate::test_support::ember::Band::Frame,
        backend.buffer(),
    );
}
