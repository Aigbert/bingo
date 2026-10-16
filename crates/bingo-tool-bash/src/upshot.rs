//! What a command came to, in the one line its step shows: how many tests
//! passed when the output says, `ok` when it does not, and how it ended when
//! that was anything but a clean exit (ADR-0095 §9).

use bingo_sdk::Upshot;

use crate::output::Ended;

pub fn upshot(body: &str, ended: Ended) -> Upshot {
    match ended {
        Ended::Exited(0) => passed(body)
            .map(|count| Upshot::good(format!("{count} passed")))
            .unwrap_or_else(|| Upshot::good("ok")),
        Ended::Exited(code) => Upshot::bad(format!("exit {code}")),
        Ended::Timeout { .. } => Upshot::bad("timed out"),
        Ended::Interrupted => Upshot::bad("stopped"),
    }
}

/// Tests passed, summed over every summary line a runner printed: cargo's
/// `test result:` (one per test binary), jest's `Tests:` and pytest's
/// `=== 3 passed in 0.1s ===`. A line that only mentions the word elsewhere is
/// not a count.
fn passed(body: &str) -> Option<u32> {
    let counts: Vec<u32> = body
        .lines()
        .map(str::trim)
        .filter(|line| summary(line))
        .filter_map(count_before_passed)
        .collect();
    (!counts.is_empty()).then(|| counts.iter().sum())
}

fn summary(line: &str) -> bool {
    line.starts_with("test result:") || line.starts_with("Tests:") || line.starts_with('=')
}

fn count_before_passed(line: &str) -> Option<u32> {
    let words: Vec<&str> = line.split_whitespace().collect();
    words.windows(2).find_map(|pair| {
        let word = pair[1].trim_end_matches([',', ';']);
        (word == "passed").then(|| pair[0].parse().ok()).flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_runners_summary_is_counted_across_binaries() {
        for (body, expected) in [
            (
                "running 3 tests\ntest result: ok. 3 passed; 0 failed; 0 ignored\n\
                 test result: ok. 2 passed; 0 failed; 0 ignored",
                "5 passed",
            ),
            (
                "tests/test_parse.py ...\n===== 3 passed in 0.02s =====",
                "3 passed",
            ),
            (
                "Test Suites: 1 passed, 1 total\nTests:       4 passed, 4 total",
                "4 passed",
            ),
            ("built\n", "ok"),
            ("the 2 passed checks are logged", "ok"),
        ] {
            assert_eq!(
                upshot(body, Ended::Exited(0)),
                Upshot::good(expected),
                "{body}"
            );
        }
    }

    #[test]
    fn an_unclean_end_is_bad_news_whatever_the_output_says() {
        let body = "test result: FAILED. 2 passed; 1 failed";
        assert_eq!(upshot(body, Ended::Exited(101)), Upshot::bad("exit 101"));
        assert_eq!(
            upshot("", Ended::Timeout { after_ms: 1_000 }),
            Upshot::bad("timed out")
        );
        assert_eq!(upshot("tick", Ended::Interrupted), Upshot::bad("stopped"));
    }
}
