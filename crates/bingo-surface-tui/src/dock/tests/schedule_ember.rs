use super::ember_tests::{fixture, show};
use crate::test_support::{
    ember::{Band, Capture, assert_regions},
    *,
};
use bingo_services::schedule::{Listing, Row};
#[test]
fn m198_schedule_full_frames() {
    for (name, paused) in [("schedule", false), ("schedule-paused", true)] {
        let capture = Capture::load(name);
        capture.render(|| {
            let (mut state, mut ui) = fixture(&capture);
            let now = capture.now();
            let zone = jiff::tz::TimeZone::UTC;
            let tomorrow = now
                .wall
                .to_zoned(zone.clone())
                .date()
                .checked_add(jiff::Span::new().days(1))
                .unwrap()
                .at(9, 0, 0, 0)
                .to_zoned(zone)
                .unwrap()
                .timestamp();
            let listing = Listing {
                entries: crate::test_support::ember::world::rows("scheds")
                    .into_iter()
                    .enumerate()
                    .map(|(i, row)| Row {
                        id: i.to_string(),
                        spec: row["when"].as_str().unwrap().into(),
                        text: row["what"].as_str().unwrap().into(),
                        enabled: row["on"].as_bool().unwrap() && !(i == 0 && paused),
                        next_fire: match row["next"].as_str().unwrap() {
                            "in 12m" => {
                                Some((now.wall + jiff::SignedDuration::from_mins(12)).to_string())
                            }
                            "tomorrow 09:00" => Some(tomorrow.to_string()),
                            _ => None,
                        },
                        by: "you".into(),
                        mode: None,
                        session: None,
                    })
                    .collect(),
                ..Default::default()
            };
            show(
                &mut state,
                &mut ui,
                "bingo.mod.schedule",
                bingo_mod_schedule::draw::tree(
                    &listing,
                    listing.entries.first().map(|row| row.id.as_str()),
                    &jiff::tz::TimeZone::UTC,
                    100,
                    19,
                    now.wall,
                    Default::default(),
                ),
            );
            if paused {
                ui.notify(
                    bingo_sdk::Level::Info,
                    "paused · 看一眼 CI，挂了就告诉我",
                    now.instant - std::time::Duration::from_millis(400),
                );
            }
            let screen = drawn(100, 32, &solo(&state), &mut ui, now);
            assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
        });
    }
}
