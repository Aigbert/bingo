//! The structured headline a person sees instead of the model notice.
use super::tone;
use crate::jobs::{Job, State};

pub fn headline(job: &Job, state: State, outcome: &str) -> bingo_sdk::Headline {
    bingo_sdk::Headline {
        tone: tone(state),
        parts: vec![
            format!(
                "{} finished in the background",
                code(&crate::jobs::head(&job.command))
            ),
            plain(outcome),
        ],
    }
}

fn code(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    let pad = if text.starts_with('`')
        || text.ends_with('`')
        || (text.starts_with(' ') && text.ends_with(' '))
    {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{text}{pad}{fence}")
}

fn plain(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_parts_keep_backticks_and_outcome_punctuation() {
        assert_eq!(code("echo `date`"), "`` echo `date` ``");
        assert_eq!(code("`date` now"), "`` `date` now ``");
        assert_eq!(code(" padded "), "`  padded  `");
        assert_eq!(plain("2 *ok*_ [done]"), r"2 \*ok\*\_ \[done\]");
    }
}
