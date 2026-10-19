use super::ember_tests::{fixture, show};
use crate::test_support::{
    ember::{Band, Capture, assert_regions},
    *,
};
use bingo_sdk::*;
use serde_json::json;

fn summaries(now: jiff::Timestamp) -> Vec<SessionSummary> {
    crate::test_support::ember::world::rows("sessions")
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let mut summary = state().summary;
            summary.id = SessionId::from_raw(format!("session-{i}"));
            summary.title = Some(row["t"].as_str().unwrap().into());
            summary.cwd = row["dir"].as_str().unwrap().into();
            summary.asking = row["st"] == "asking";
            summary.busy = row["st"] == "working";
            summary.updated_at = when(row["when"].as_str().unwrap(), now);
            summary
        })
        .collect()
}
fn when(word: &str, now: jiff::Timestamp) -> jiff::Timestamp {
    if let Some(minutes) = word.strip_suffix('m') {
        return now - jiff::SignedDuration::from_mins(minutes.parse().unwrap());
    }
    if let Some(hours) = word.strip_suffix('h') {
        return now - jiff::SignedDuration::from_hours(hours.parse().unwrap());
    }
    let (month, day) = word.split_once(' ').unwrap();
    format!(
        "1969-{:02}-{:02}T00:00:00Z",
        match month {
            "Sep" => 9,
            "Aug" => 8,
            _ => panic!("unknown month"),
        },
        day.parse::<u8>().unwrap()
    )
    .parse()
    .unwrap()
}
async fn owner_view(capture: &Capture, query: &str) -> View {
    let home = tempfile::tempdir().unwrap();
    let mut registrar = Registrar::new(
        "sessions",
        json!({}),
        Env::rooted(home.path()),
        Default::default(),
    );
    bingo_mod_sessions::Sessions
        .register(&mut registrar)
        .unwrap();
    let link = registrar
        .into_contributions()
        .into_iter()
        .find_map(|c| match c {
            Contribution::Interceptor { link, .. } if link.id() == "sessions" => Some(link),
            _ => None,
        })
        .unwrap();
    let session = state().summary.id;
    let props = component::SessionsProps {
        session: session.clone(),
        cwd: "~/bingo-improve".into(),
        query: query.into(),
        scope: component::SessionsScope::All,
        limit: 100,
        sessions: summaries(capture.now().wall),
        now: capture.now().wall,
        action: Action {
            name: "surface:sessions".into(),
            args: json!({}),
        },
    };
    let host = testing::TestHost::default();
    let request = render::RenderRequest {
        component: render::Component::Sessions,
        request_id: "sessions".into(),
        surface: render::SurfaceName::Terminal,
        props: serde_json::to_value(props).unwrap(),
        viewport: render::Viewport {
            glyphs: Default::default(),
            columns: 100,
            rows: 19,
            place: None,
        },
    };
    let cx = HookContext {
        session,
        turn: None,
        item: None,
        cwd: home.path().into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    serde_json::from_value(
        link.chain(
            catalogue::UI_RENDER,
            &cx,
            serde_json::to_value(request).unwrap(),
            Next::new(|v| Box::pin(async { Ok(v) })),
        )
        .await
        .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn m198_sessions_full_frames() {
    for (name, query) in [("sessions", ""), ("sessions-filtered", "ci")] {
        let capture = Capture::load(name);
        let view = owner_view(&capture, query).await;
        capture.render(|| {
            let (mut state, mut ui) = fixture(&capture);
            show(&mut state, &mut ui, "bingo.mod.sessions", view);
            let screen = drawn(100, 32, &solo(&state), &mut ui, capture.now());
            assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
        });
    }
}
