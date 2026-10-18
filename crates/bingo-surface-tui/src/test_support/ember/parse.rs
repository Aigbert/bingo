use super::{Row, Run};
use unicode_width::UnicodeWidthChar;

pub(super) fn rows(text: &str, inks: &str) -> Vec<Row> {
    let mut rows: Vec<_> = text
        .lines()
        .skip(1)
        .map(|line| Row {
            cells: cells(line),
            ..Default::default()
        })
        .collect();
    for line in inks.lines().filter(|line| !line.starts_with('#')) {
        let mut parts = line.splitn(3, ' ');
        let y: usize = parts.next().unwrap().parse().unwrap();
        let layer = parts.next().unwrap();
        let runs = parts.next().unwrap().split(" · ").map(run).collect();
        match layer {
            "fg" => rows[y].fg = runs,
            "bg" => rows[y].bg = runs,
            _ => panic!("ink layer {layer}"),
        }
    }
    rows
}

fn cells(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ch in line.chars() {
        let width = ch.width().unwrap_or(0);
        if width == 0 {
            if let Some(last) = out.last_mut() {
                last.push(ch);
            }
        } else {
            out.push(ch.to_string());
            out.extend((1..width).map(|_| String::new()));
        }
    }
    out
}

fn run(text: &str) -> Run {
    let (range, token) = text.split_once(' ').unwrap();
    let (start, end) = range.split_once('-').unwrap_or((range, range));
    Run {
        start: start.parse().unwrap(),
        end: end.parse().unwrap(),
        token: token.into(),
    }
}

pub(super) fn token(runs: &[Run], x: u16) -> Option<&str> {
    runs.iter()
        .find(|run| (run.start..=run.end).contains(&x))
        .map(|run| run.token.as_str())
}

pub(super) fn full_ground(row: &Row, width: u16) -> bool {
    (0..width).all(|x| token(&row.bg, x).is_some_and(|token| token != "bg"))
}
