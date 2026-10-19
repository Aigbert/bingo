//! `crewScript()`, `scoutScript()` and `reviewerScript()` of p4b-data.js and
//! p4d-crew.js: home hands two tasks out, each teammate works in its own
//! session, and a finished one reports back home.
use super::script::{Beat, Turn};
use super::world::{self, id};
use crate::test_support::*;
use bingo_sdk::{
    AnswerSpec, Event, Interaction, InteractionId, InteractionKind, ItemId, Preview, SessionId,
    TurnId,
};

/// `CREW_P` in p8-page.js.
pub const CREW_P: &str =
    "让 scout 和 reviewer 并行：一个找 drain() 的调用点，一个审 queue.rs 的改动";

const CREW_SAYS: &str = "派出去了：**scout** 找所有调用 `drain()` 的地方，**reviewer** 看 `queue.rs` 的改动。它们做完会在这里说；要你点头的时候，页脚会告诉你，`ctrl+g` 过去。";

const SCOUT_SAYS: &str = "找到 4 处调用 `drain()`：`turn/loop.rs` 两处、`session/mod.rs` 一处、`rpc/steer.rs` 一处。都只在一轮开始时调用，和 `reset()` 的顺序一致，改动不影响它们。";

/// `REVIEWER_SAYS` in p4b-data.js: the review, once the tests ran.
const REVIEWER_REPORTS: &str = "`reset()` 的改法没问题。建议在 `a_second_steer_replaces_the_first` 里再断言一次队列是空的，不然以后有人把 steer 改成追加，这个测试还是会过。";

/// When the reviewer asks, in its turn's time.
const ASKS_AT: i64 = 1400;

const QF: &str = "crates/bingo-core/src/session/queue.rs";
const TF: &str = "crates/bingo-core/tests/queue_steer.rs";
const TEST: &str = "cargo test -p bingo-core queue";

/// A teammate taking a task: when its session opens, and when its turn
/// starts, both from the scene's setup.
pub struct Hire {
    pub opened: i64,
    pub t0: i64,
}

/// Home's turn from `t0`, with each teammate's turn as `hires` place it.
pub fn crew(home: &SessionId, t0: i64, scout: Hire, reviewer: Hire) -> Vec<Beat> {
    let mut turn = Turn::start(home, "trn_crew", t0, user("crew-prompt", CREW_P));
    turn.think(
        0,
        600,
        "两件事可以并行：scout 去找调用点，reviewer 看 diff。",
    );
    let call = turn.step(700, "agent", "find every caller of drain()");
    turn.ok(900, "started");
    let mut beats = scout_turn(home, &call, scout);
    let call = turn.step(1000, "agent", "review the queue.rs diff");
    turn.ok(1200, "started");
    beats.extend(reviewer_turn(home, &call, reviewer));
    turn.reply(1400, CREW_SAYS);
    beats.extend(turn.beats);
    beats
}

/// `hire()`: the teammate's session opens under the call, with its role,
/// and starts on the task as `bingo`'s words.
fn hired(home: &SessionId, name: &str, call: &str, task: &str, hire: &Hire) -> Turn {
    let session = id(name);
    let opened = world::SETUP + hire.opened;
    let mut turn = Turn::start(
        &session,
        &format!("trn_{name}"),
        hire.t0,
        voiced(
            delivered(&format!("{name}-task"), "agent", None, task),
            bingo_sdk::Voice::Quiet,
        ),
    );
    let opening = [
        world::hired(home, name, opened, Some(call)),
        world::role(name),
    ];
    let first = opening.into_iter().map(|event| Beat {
        t: hire.opened,
        session: session.clone(),
        event,
    });
    turn.beats.splice(0..0, first);
    turn
}

fn scout_turn(home: &SessionId, call: &str, hire: Hire) -> Vec<Beat> {
    let mut turn = hired(home, "scout", call, "find every caller of drain()", &hire);
    turn.think(0, 500, "先 grep 调用点，再逐个读上下文。");
    turn.step(600, "grep", "drain\\(");
    turn.ok(900, "4 hits in 3 files");
    turn.step(1000, "read", "crates/bingo-core/src/turn/loop.rs");
    turn.ok(1200, "312 lines");
    turn.step(1300, "read", "crates/bingo-core/src/session/mod.rs");
    turn.ok(1450, "188 lines");
    turn.step(1550, "read", "crates/bingo-rpc/src/steer.rs");
    turn.ok(1700, "96 lines");
    let end = turn.reply(1900, SCOUT_SAYS);
    turn.call(
        end,
        home,
        report("scout", SCOUT_SAYS, world::SETUP + hire.t0 + end),
    );
    turn.beats
}

fn reviewer_turn(home: &SessionId, call: &str, hire: Hire) -> Vec<Beat> {
    let mut turn = hired(home, "reviewer", call, "review the queue.rs diff", &hire);
    turn.think(0, 600, "先读改动，再跑一遍队列的测试。");
    turn.step(700, "read", QF);
    turn.ok(900, "214 lines");
    turn.step(1000, "read", TF);
    turn.ok(1150, "2 tests");
    let bash = turn.step(1300, "bash", TEST);
    let ask = asked(&bash, world::SETUP + hire.t0 + 1400);
    turn.call(
        1400,
        &id("reviewer"),
        Event::InteractionOpened { interaction: ask },
    );
    turn.beats
}

/// The rest of `reviewerScript()` once its ask is answered at `answered`,
/// for a turn that started at `t0`: the run lands, the review is written and
/// reported home.
pub fn reviewer_resumes(home: &SessionId, t0: i64, answered: i64) -> Vec<Beat> {
    let pause = (ASKS_AT, answered - t0 - ASKS_AT);
    let mut turn = Turn::resumed(&id("reviewer"), "trn_reviewer", t0, 3, pause);
    turn.reopen(1300, "bash", TEST);
    turn.waited();
    turn.ok(3000, "12 passed");
    let end = turn.reply(3200, REVIEWER_REPORTS);
    let at = world::SETUP + t0 + pause.1 + end;
    turn.call(end, home, report("reviewer", REVIEWER_REPORTS, at));
    turn.beats
}

/// `reportBack()`: the teammate's answer, posted home in its name, as the
/// agents plugin's report signs it.
fn report(name: &str, text: &str, at: i64) -> Event {
    let mut event = world::post_event(&format!("{name}-report"), name, text, at);
    if let Event::ItemCompleted { item } = &mut event
        && let bingo_sdk::ItemBody::User { origin, .. } = &mut item.body
    {
        *origin = bingo_sdk::Origin {
            render_surface: None,
            voice: Some(bingo_sdk::Voice::Quiet),
            surface: bingo_collab::name::SURFACE.into(),
            principal: Some(name.into()),
            conversation: None,
        };
    }
    event
}

/// The reviewer's `B.ask(1400, 'bash', …)`: a run waits on you.
fn asked(call: &str, at: i64) -> Interaction {
    let home = crate::paths::home().unwrap_or("/home/me");
    Interaction {
        id: InteractionId::from_raw("int_reviewer"),
        session: id("reviewer"),
        turn: Some(TurnId::from_raw("trn_reviewer")),
        item: Some(ItemId::from_raw(call)),
        opened_at: jiff::Timestamp::from_millisecond(at).unwrap(),
        guard_until: None,
        expires_at: None,
        kind: InteractionKind::Permission {
            tool: "bash".into(),
            summary: TEST.into(),
            preview: Some(Box::new(Preview::Command {
                command: TEST.into(),
                cwd: format!("{home}/bingo-improve"),
                description: Some("checks the fix before I sign off".into()),
                stakes: None,
            })),
            session_scope: Some("Bash(cargo test:*)".into()),
            caution: None,
            input: Default::default(),
        },
        answers: vec![
            AnswerSpec::AllowOnce,
            AnswerSpec::AllowSession,
            AnswerSpec::Deny,
            AnswerSpec::AllowChanged,
        ],
    }
}
