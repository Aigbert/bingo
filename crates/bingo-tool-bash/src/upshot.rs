//! What a command came to, in the one line its step shows: how many tests
//! passed when the output says, `ok` when it does not, and how it ended when
//! that was anything but a clean exit (ADR-0095 §9).

use bingo_sdk::Upshot;

use crate::output::Ended;

/// `whole` says `body` is everything the command wrote: a count summed over
/// output cut in the middle would undercount, in good ink.
pub fn upshot(body: &str, whole: bool, ended: Ended) -> Upshot {
    match ended {
        Ended::Exited(0) => whole
            .then(|| passed(body))
            .flatten()
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
/// not a count, and nothing passed is not news.
fn passed(body: &str) -> Option<u32> {
    let total: u32 = body
        .lines()
        .map(str::trim)
        .filter(|line| summary(line))
        .filter_map(count_before_passed)
        .sum();
    (total > 0).then_some(total)
}

fn summary(line: &str) -> bool {
    line.starts_with("test result:") || line.starts_with("Tests:") || pytest(line)
}

/// pytest's closing rule: the counts between two runs of `=`.
fn pytest(line: &str) -> bool {
    line.len() > 2 && line.starts_with("==") && line.ends_with("==")
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
            ("= 2 passed, not a rule", "ok"),
            ("test result: ok. 0 passed; 0 failed; 0 ignored", "ok"),
        ] {
            assert_eq!(
                upshot(body, true, Ended::Exited(0)),
                Upshot::good(expected),
                "{body}"
            );
        }
    }

    #[test]
    fn an_unclean_end_is_bad_news_whatever_the_output_says() {
        let body = "test result: FAILED. 2 passed; 1 failed";
        assert_eq!(
            upshot(body, true, Ended::Exited(101)),
            Upshot::bad("exit 101")
        );
        assert_eq!(
            upshot("", true, Ended::Timeout { after_ms: 1_000 }),
            Upshot::bad("timed out")
        );
        assert_eq!(
            upshot("tick", true, Ended::Interrupted),
            Upshot::bad("stopped")
        );
    }

    /// Output cut in the middle lost summary lines with it: whatever is left
    /// would undercount, so the step says only that it went well.
    #[test]
    fn output_cut_in_the_middle_is_not_counted() {
        let body = "test result: ok. 3 passed; 0 failed\n[… 9 chars truncated …]\n\
                    test result: ok. 2 passed; 0 failed";
        assert_eq!(upshot(body, false, Ended::Exited(0)), Upshot::good("ok"));
    }
}
