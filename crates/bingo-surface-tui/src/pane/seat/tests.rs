use super::*;
use bingo_sdk::slot::{Anchor, Length, PaneSize, Rect as Declaration, Span};

fn pane(key: &str, place: Vec<Place>) -> Pane {
    Pane {
        key: key.into(),
        plugin: "counter".into(),
        id: key.into(),
        title: key.into(),
        focus: true,
        close: true,
        asked: true,
        place,
        size: PaneSize {
            rows: Span {
                min: Length::Cells(8),
                ideal: Length::Cells(10),
                max: Length::Cells(20),
            },
            columns: Span {
                min: Length::Cells(48),
                ideal: Length::Cells(64),
                max: Length::Cells(80),
            },
        },
        view: bingo_sdk::View::text("counter").into(),
    }
}
#[test]
fn first_fitting_place_preserves_the_transcript_minimum() {
    let wanted = [pane("counter", vec![Place::Side, Place::Above])];
    for (name, width, height, expected, transcript) in [
        ("wide", 200, 50, Some(Place::Side), Rect::new(0, 0, 136, 50)),
        (
            "medium",
            100,
            30,
            Some(Place::Above),
            Rect::new(0, 0, 100, 20),
        ),
        ("small", 60, 15, None, Rect::new(0, 0, 60, 15)),
    ] {
        let got = seat(Rect::new(0, 0, width, height), &wanted, &BTreeMap::new());
        assert_eq!(got.panes.first().map(|pane| pane.place), expected, "{name}");
        assert_eq!(got.transcript, transcript, "{name}");
        assert_eq!(got.collapsed.is_empty(), expected.is_some(), "{name}");
    }
}
#[test]
fn anchors_resolve_cells_and_percentages_and_move_before_shrinking() {
    for percentage in [false, true] {
        for (anchor, x, y) in [
            (Anchor::TopLeft, 10, 5),
            (Anchor::TopRight, 50, 5),
            (Anchor::BottomLeft, 10, 30),
            (Anchor::BottomRight, 50, 30),
            (Anchor::Center, 40, 22),
        ] {
            let length = |n| {
                if percentage {
                    Length::Percent(n)
                } else {
                    Length::Cells(u16::from(n))
                }
            };
            let rect = Declaration {
                anchor,
                x: Length::Cells(10),
                y: Length::Cells(5),
                width: length(40),
                height: if percentage { length(30) } else { length(15) },
            };
            let mut wanted = pane("corner", vec![Place::Rect(rect)]);
            wanted.size.columns.min = Length::Cells(1);
            let got = seat(Rect::new(2, 3, 100, 50), &[wanted], &BTreeMap::new());
            assert_eq!(
                got.panes[0].area,
                Rect::new(x + 2, y + 3, 40, 15),
                "{anchor:?} {percentage}"
            );
        }
    }
    for (name, x, width, min, expected) in [
        ("past edge", 95, 40, 1, Some(Rect::new(60, 0, 40, 20))),
        ("oversize", 95, 140, 1, Some(Rect::new(0, 0, 100, 20))),
        ("under min", 95, 140, 120, None),
    ] {
        let mut wanted = pane(
            "corner",
            vec![Place::Rect(Declaration {
                anchor: Anchor::TopLeft,
                x: Length::Cells(x),
                y: Length::Cells(0),
                width: Length::Cells(width),
                height: Length::Cells(20),
            })],
        );
        wanted.size.columns.min = Length::Cells(min);
        wanted.size.columns.max = Length::Cells(200);
        let got = seat(Rect::new(0, 0, 100, 50), &[wanted], &BTreeMap::new());
        assert_eq!(got.panes.first().map(|pane| pane.area), expected, "{name}");
    }
}
#[test]
fn occupied_places_and_floating_limit_try_the_next_preference() {
    let wanted = [
        pane("first", vec![Place::Side]),
        pane("second", vec![Place::Side, Place::Above]),
    ];
    let got = seat(Rect::new(0, 0, 200, 50), &wanted, &BTreeMap::new());
    assert_eq!(
        got.panes.iter().map(|pane| pane.place).collect::<Vec<_>>(),
        vec![Place::Side, Place::Above]
    );
    assert_eq!(got.transcript, Rect::new(0, 0, 136, 40));
    let wanted = (0..5)
        .map(|n| pane(&n.to_string(), vec![Place::Overlay, Place::Above]))
        .collect::<Vec<_>>();
    let got = seat(Rect::new(0, 0, 200, 50), &wanted, &BTreeMap::new());
    assert_eq!(got.panes.len(), FLOATING_MAX + 1);
    assert_eq!(got.panes[3].place, Place::Above);
    assert_eq!(got.collapsed, vec!["4"]);
}
#[test]
fn the_persons_move_wins_while_it_fits() {
    let wanted = [pane("counter", vec![Place::Above])];
    let moved = BTreeMap::from([("counter".into(), Place::Side)]);
    assert_eq!(
        seat(Rect::new(0, 0, 200, 50), &wanted, &moved).panes[0].place,
        Place::Side
    );
    assert_eq!(
        seat(Rect::new(0, 0, 100, 30), &wanted, &moved).panes[0].place,
        Place::Above
    );
}

#[test]
fn an_above_pane_preserves_the_existing_side_panes_minimum() {
    let mut side = pane("side", vec![Place::Side]);
    side.size.rows.min = Length::Cells(45);
    side.size.rows.max = Length::Cells(50);
    let above = pane("above", vec![Place::Above]);
    let got = seat(Rect::new(0, 0, 200, 50), &[side, above], &BTreeMap::new());
    assert_eq!(got.panes.len(), 1);
    assert_eq!(got.collapsed, vec!["above"]);
}
