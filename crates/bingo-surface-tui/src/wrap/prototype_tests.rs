use super::*;
use crate::theme;

#[test]
fn every_wraptest_case_matches_word_for_word() {
    let line = Line::from(vec![
        Span::raw("把返回值改成拥有所有权的 "),
        Span::styled("Message", theme::presence()),
        Span::raw(" 就行，"),
        Span::styled("pop_front()", theme::presence()),
        Span::raw(" 本来就把所有权交给你；或者在调用处先 "),
        Span::styled("clone()", theme::presence()),
        Span::raw("。"),
    ]);
    for &(width, expected) in SCREENSHOT {
        assert_rows(&line, width, expected);
    }
}

#[test]
fn brackets_styled_words_long_units_and_newlines_match_the_prototype() {
    assert_rows(
        &Line::from("所以只有「插话之后紧跟着排队」才会丢。"),
        13,
        &["所以只有「插", "话之后紧跟着", "排队」才会", "丢。"],
    );
    assert_rows(
        &Line::from(vec![
            Span::raw("abc "),
            Span::styled("queue", theme::presence()),
            Span::raw("s are"),
        ]),
        8,
        &["abc", "queues", "are"],
    );
    assert_rows(
        &Line::from("averyveryverylongidentifier_without_breaks"),
        10,
        &["averyveryv", "erylongide", "ntifier_wi", "thout_brea", "ks"],
    );
    assert_rows(
        &Line::from(vec![Span::raw("line one\nline two")]),
        40,
        &["line one", "line two"],
    );
}
fn assert_rows(line: &Line<'static>, width: usize, expected: &[&str]) {
    let rows: Vec<_> = wrap(line, width).iter().map(ToString::to_string).collect();
    assert_eq!(rows, expected, "width {width}");
}

// Generated once from wraptest.js; the test needs no JavaScript runtime.
const SCREENSHOT: &[(usize, &[&str])] = &[
    (
        60,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把",
            "所有权交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        61,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所",
            "有权交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        62,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所",
            "有权交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        63,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有",
            "权交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        64,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有",
            "权交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        65,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有权",
            "交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        66,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有权",
            "交给你；或者在调用处先 clone()。",
        ],
    ),
    (
        67,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有权交",
            "给你；或者在调用处先 clone()。",
        ],
    ),
    (
        68,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有权交",
            "给你；或者在调用处先 clone()。",
        ],
    ),
    (
        69,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有权交给",
            "你；或者在调用处先 clone()。",
        ],
    ),
    (
        70,
        &[
            "把返回值改成拥有所有权的 Message 就行，pop_front() 本来就把所有权交给",
            "你；或者在调用处先 clone()。",
        ],
    ),
];
