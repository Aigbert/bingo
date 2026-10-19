//! M200's full frames: the crew, a teammate's session and a room, each
//! driven by the prototype's keys and compared cell by cell and ink by ink.
use crate::test_support::ember::{self, Band, Capture};
use crate::test_support::*;
use crate::{clock::Now, effect::Effect, ui::Ui};
use bingo_sdk::{Event, Input, SessionId};
use crossterm::event::{KeyCode, KeyEvent};
use world::World;

mod crew;
mod script;
mod world;

fn ui() -> Ui {
    let (mut ui, _) = scene();
    ui.catalogs
        .values
        .insert("tools".into(), ember::story::catalogue());
    ui
}

/// `past(); designRoom(); enterView('#design')`.
fn room_world() -> World {
    let mut world = World::home();
    world.mate("scout", world::SETUP - 600_000);
    world.mate("reviewer", world::SETUP - 599_000);
    world.mate("ada", world::SETUP - 598_000);
    world.design_room();
    world
}

/// `S.mode = 1; past(); designRoom(); startTurn(CREW_P, crewScript())`:
/// each hire lands at the first 50 ms tick after its beat.
fn crew_beats(world: &World) -> Vec<script::Beat> {
    crew::crew(
        world.tree.root_id(),
        0,
        crew::Hire {
            opened: 750,
            t0: 750,
        },
        crew::Hire {
            opened: 1050,
            t0: 1050,
        },
    )
}

/// `S.log.push(simulate(CREW_P, crewScript())); ago('scout', 600);
/// ago('reviewer', 2600)`: home's turn is long over, each teammate opened at
/// its call and runs from moments before the scene.
fn agent_beats(world: &World) -> Vec<script::Beat> {
    const LONG_AGO: i64 = -10_000_000;
    crew::crew(
        world.tree.root_id(),
        LONG_AGO,
        crew::Hire {
            opened: LONG_AGO + 700,
            t0: -600,
        },
        crew::Hire {
            opened: LONG_AGO + 1000,
            t0: -2600,
        },
    )
}

/// What the kernel does later on its own: an event on a session, at a
/// moment of the scene.
struct Due {
    at: i64,
    session: SessionId,
    event: Box<dyn FnOnce(i64) -> Event>,
}

/// One run of a scene: its world, the surface's own state and the clock.
struct Drive {
    world: World,
    ui: Ui,
    now: Now,
    size: (u16, u16),
    /// Milliseconds since the scene's setup.
    t: i64,
    due: Vec<Due>,
    posts: usize,
    /// What a teammate does once its ask is answered, from that moment.
    resumes: Option<Box<dyn FnOnce(i64) -> Vec<script::Beat>>>,
}

impl Drive {
    fn new(world: World, capture: &Capture) -> Self {
        let size = capture.size();
        let mut now = capture.now();
        now.wall = jiff::Timestamp::from_millisecond(world::SETUP).unwrap();
        let mut drive = Self {
            world,
            ui: ui(),
            now,
            size: (size.width, size.height),
            t: 0,
            due: Vec::new(),
            posts: 0,
            resumes: None,
        };
        drive.draw();
        drive
    }

    /// A frame, with each ledger row it pulls answered by the crew's rows,
    /// which hand every other step on to the ledger's own drawing.
    fn draw(&mut self) -> ratatui::backend::TestBackend {
        let screen = self.drawn();
        if self.answer_pulls() {
            return self.drawn();
        }
        screen
    }

    fn drawn(&mut self) -> ratatui::backend::TestBackend {
        drawn(
            self.size.0,
            self.size.1,
            &self.world.tree,
            &mut self.ui,
            self.now,
        )
    }

    fn answer_pulls(&mut self) -> bool {
        let mut answered = false;
        for key in self.ui.shown.pulls.clone() {
            let parsed = bingo_sdk::component::Props::parse(
                key.request.component,
                key.request.props.clone(),
            );
            let Ok(bingo_sdk::component::Props::Ledger(props)) = parsed else {
                continue;
            };
            if !self.ui.placed.pulled.request(&key) {
                continue;
            }
            let width = usize::from(key.request.viewport.columns);
            self.ui
                .placed
                .pulled
                .answer(key, bingo_mod_crew::rows::ledger(&props, width));
            answered = true;
        }
        answered
    }

    /// A script's beats: what is already past folds now, the rest when due.
    fn script(&mut self, mut beats: Vec<script::Beat>) {
        beats.sort_by_key(|beat| beat.t);
        for beat in beats {
            if beat.t <= self.t {
                self.world
                    .emit(&beat.session, world::SETUP + beat.t, beat.event);
            } else {
                let event = beat.event;
                self.due.push(Due {
                    at: beat.t,
                    session: beat.session,
                    event: Box::new(move |_| event),
                });
            }
        }
    }

    /// What the loop does with `Effect::View`.
    fn show(&mut self, session: &bingo_sdk::SessionId) {
        self.world.tree.show(session);
        self.ui.switched = Some(self.now.instant);
        self.ui.scroll = Default::default();
        let open = self.world.tree.open_interaction().map(|(_, open)| open);
        self.ui.dialog.focus_on(open);
        self.draw();
    }

    /// Let `ms` pass as the prototype's capture does, in ticks of at most
    /// 50 ms: whatever fell due folds at the first tick that reaches it.
    fn wait(&mut self, ms: i64) {
        let mut left = ms;
        while left > 0 {
            let step = left.min(50);
            self.to(self.t + step);
            left -= step;
            self.due.sort_by_key(|due| due.at);
            while self.due.first().is_some_and(|due| due.at <= self.t) {
                let due = self.due.remove(0);
                let at = world::SETUP + self.t;
                self.world.emit(&due.session, at, (due.event)(at));
            }
            self.draw();
        }
    }

    fn to(&mut self, t: i64) {
        self.now = later(self.now, t - self.t);
        self.t = t;
    }

    fn later(
        &mut self,
        after: i64,
        session: &SessionId,
        event: impl FnOnce(i64) -> Event + 'static,
    ) {
        self.due.push(Due {
            at: self.t + after,
            session: session.clone(),
            event: Box::new(event),
        });
    }

    /// One key, as the prototype presses it: it lands, then 60 ms pass.
    fn press(&mut self, key: KeyEvent) {
        let effects = crate::input::on_key(&mut self.ui, &self.world.tree, key, self.now);
        for effect in effects {
            self.answer(effect);
        }
        self.draw();
        self.wait(60);
    }

    /// The kernel's side of what a key asked for.
    fn answer(&mut self, effect: Effect) {
        match effect {
            Effect::View(session) => self.show(&session),
            Effect::Submit(Input::Text { text, .. }) => self.posted(&text),
            Effect::Answer {
                interaction,
                answer,
                ..
            } => self.answered(interaction, answer),
            other => panic!("the scene has no answer for {other:?}"),
        }
    }

    /// The kernel resolves the ask in the viewed session, and the teammate
    /// goes on.
    fn answered(&mut self, id: bingo_sdk::InteractionId, answer: bingo_sdk::Answer) {
        let session = self.world.tree.viewed().summary.id.clone();
        let by = bingo_sdk::ResolvedBy::Client {
            name: "tui".into(),
            surface: "tui".into(),
        };
        let event = Event::InteractionResolved { id, answer, by };
        self.world.emit(&session, world::SETUP + self.t, event);
        if let Some(resumes) = self.resumes.take() {
            self.script(resumes(self.t));
        }
    }

    /// A line sent in the room is a post; each teammate it calls on writes,
    /// then answers, as `post()` in p4d-crew.js has it.
    fn posted(&mut self, text: &str) {
        let room = self.world.tree.viewed().summary.id.clone();
        self.posts += 1;
        let at = world::SETUP + self.t;
        self.world
            .post(&room, &format!("you-{}", self.posts), "parent", text, at);
        if text.contains("@reviewer") {
            let mate = world::id("reviewer");
            self.later(700, &mate, |_| started("trn_room"));
            self.later(2600, &room, |at| {
                world::post_event("room-reviewer", "reviewer", world::REVIEWER_SAYS, at)
            });
            self.later(2600, &mate, |_| {
                completed("trn_room", bingo_sdk::TurnStatus::Completed)
            });
        }
    }

    /// The capture's `after`: waits, typed text and named keys.
    fn play(&mut self, capture: &Capture) {
        for step in &capture.entry.after {
            if let Some(ms) = step.as_i64() {
                self.wait(ms);
            } else if let Some(text) = step.get(0).and_then(|text| text.as_str()) {
                text.chars().for_each(|c| self.press(typed(c)));
            } else if let Some(chord) = step.as_str().filter(|s| !s.starts_with("shot:")) {
                self.press(chord_key(chord));
            }
        }
    }

    fn shot(&mut self, capture: &Capture) {
        let screen = self.draw();
        ember::assert_regions(
            capture,
            Band::Frame,
            screen.buffer(),
            &self.ui.shown.regions,
        );
    }
}

fn chord_key(chord: &str) -> KeyEvent {
    match chord {
        "enter" => key(KeyCode::Enter),
        "esc" => key(KeyCode::Esc),
        "tab" => key(KeyCode::Tab),
        "ctrl+g" => ctrl('g'),
        _ => panic!("{chord}"),
    }
}

/// The capture `name` driven by the prototype's keys, compared full frame.
pub(crate) fn full_frame(name: &str) {
    match name {
        "room" => room_frame(),
        "room-mention" | "room-answered" | "room-home" => room_keys(name, &[]),
        "room-waiting" => room_keys(name, &[(26, 14)]),
        "crew" | "small-crew" => crew_still(name),
        "agent" => agent_still(),
        "crew-reviewer" | "crew-allowed" | "crew-home" => crew_frame(name, false),
        "agent-reviewer" | "agent-allowed" | "agent-home" => crew_frame(name, true),
        _ => panic!("{name} is not a crew capture"),
    }
}

fn room_frame() {
    let capture = Capture::load("room");
    capture.render(|| {
        let mut drive = Drive::new(room_world(), &capture);
        drive.show(&world::id("design"));
        drive.wait(capture.entry.moment_ms);
        drive.shot(&capture);
    });
}

fn room_keys(name: &str, spinners: &[(u16, u16)]) {
    let capture = Capture::load(name).spinners(spinners);
    capture.render(|| {
        let mut drive = Drive::new(room_world(), &capture);
        drive.show(&world::id("design"));
        drive.play(&capture);
        assert_eq!(drive.t, capture.entry.moment_ms, "{name}");
        drive.shot(&capture);
    });
}

fn crew_still(name: &str) {
    let capture = Capture::load(name);
    capture.render(|| {
        let mut world = World::accepting_edits();
        world.design_room();
        let beats = crew_beats(&world);
        let mut drive = Drive::new(world, &capture);
        drive.script(beats);
        drive.wait(capture.entry.moment_ms);
        drive.shot(&capture);
    });
}

fn agent_still() {
    let capture = Capture::load("agent").spinners(&[(26, 3)]);
    capture.render(|| {
        let mut world = World::accepting_edits();
        world.design_room();
        let beats = agent_beats(&world);
        let mut drive = Drive::new(world, &capture);
        drive.script(beats);
        drive.show(&world::id("scout"));
        drive.wait(capture.entry.moment_ms);
        drive.shot(&capture);
    });
}

/// The capture's keys after `wait`, once the teammates are at work: ctrl+g to
/// the reviewer, allow its run, esc home.
fn crew_frame(name: &str, agent: bool) {
    let capture = Capture::load(name);
    capture.render(|| {
        let mut world = World::accepting_edits();
        world.design_room();
        let home = world.tree.root_id().clone();
        let (beats, reviewer_t0) = if agent {
            (agent_beats(&world), -2600)
        } else {
            (crew_beats(&world), 1050)
        };
        let mut drive = Drive::new(world, &capture);
        drive.script(beats);
        drive.resumes = Some(Box::new(move |at| {
            crew::reviewer_resumes(&home, reviewer_t0, at)
        }));
        if agent {
            drive.show(&world::id("scout"));
        }
        drive.play(&capture);
        assert_eq!(drive.t, capture.entry.moment_ms, "{name}");
        drive.shot(&capture);
    });
}
