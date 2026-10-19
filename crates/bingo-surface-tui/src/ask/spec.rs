//! What a permission or a confirmation asks, in the dock's words: a title,
//! what it would do, and the answers the kernel accepts, widest last.
use super::scope;
use bingo_sdk::{Answer, AnswerSpec, Interaction, InteractionKind, Preview, Stakes};

/// One run of words and the ink it is drawn in.
pub type Run = (String, &'static str);

#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub title: String,
    pub right: Vec<Run>,
    pub danger: bool,
    pub body: Body,
    pub options: Vec<Opt>,
    /// The answer the pick starts on.
    pub start: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Diff(String),
    Command {
        command: String,
        why: Option<String>,
        stakes: Option<Stakes>,
    },
    Text(String),
    Nothing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Opt {
    pub label: String,
    /// What a session answer allows, drawn as `code` while picked.
    pub scope: Option<String>,
    pub tail: &'static str,
    pub tone: Option<&'static str>,
    /// What a row that takes words says before any are typed.
    pub hint: Option<String>,
    pub does: Does,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Does {
    Send(Answer),
    /// A no, with the reason typed after it.
    Deny,
    /// The command or the replacement text, edited before it runs.
    Change,
}

impl Does {
    pub fn takes_words(&self) -> bool {
        matches!(self, Does::Deny | Does::Change)
    }
}

/// Where the ask is read: whose words a refusal goes to, the session's
/// directory and home, and the verb the tool's look gives the call.
pub struct Reading<'a> {
    pub worker: &'a str,
    pub cwd: &'a str,
    pub home: Option<&'a str>,
    pub verb: &'a str,
    pub target: &'a str,
}

pub fn ask(interaction: &Interaction, reading: &Reading<'_>) -> Option<Ask> {
    match &interaction.kind {
        InteractionKind::Permission {
            summary,
            preview,
            session_scope,
            caution,
            ..
        } => {
            let danger = caution.is_some();
            let (title, right, body) = match preview.as_deref() {
                Some(preview) => previewed(preview, reading),
                None => (
                    question(reading.verb, ""),
                    Vec::new(),
                    Body::Text(summary.clone()),
                ),
            };
            let right = match caution {
                Some(caution) => vec![(caution.reason.clone(), "bad")],
                None => right,
            };
            let act = caution.as_ref().and_then(|caution| caution.act.as_deref());
            let options = permission_options(interaction, session_scope.as_deref(), act, reading);
            let start = if danger {
                options
                    .iter()
                    .position(|o| o.does == Does::Deny)
                    .unwrap_or(0)
            } else {
                0
            };
            Some(Ask {
                title,
                right,
                danger,
                body,
                options,
                start,
            })
        }
        InteractionKind::Confirm { title, detail } => Some(Ask {
            title: title.clone(),
            right: Vec::new(),
            danger: false,
            body: Body::Text(detail.clone()),
            options: interaction
                .rungs()
                .into_iter()
                .map(|rung| plain(rung.label, Does::Send(rung.answer)))
                .collect(),
            start: 0,
        }),
        _ => None,
    }
}

fn previewed(preview: &Preview, reading: &Reading<'_>) -> (String, Vec<Run>, Body) {
    match preview {
        Preview::Diff { unified } => {
            let (added, removed) = counts(unified);
            let minus = crate::theme::glyphs::get("minus");
            (
                question(reading.verb, &tail(reading.target, 2)),
                vec![
                    (format!("+{added}"), "good"),
                    (format!(" {minus}{removed}"), "bad"),
                ],
                Body::Diff(unified.clone()),
            )
        }
        Preview::Command {
            command,
            cwd,
            description,
            stakes,
        } => (
            "Run this command?".into(),
            vec![(format!("in {}", homed(cwd, reading.home)), "dim")],
            Body::Command {
                command: command.clone(),
                why: description.clone(),
                stakes: stakes.clone(),
            },
        ),
        Preview::Url { url } => (
            format!("Fetch {}?", host(url)),
            Vec::new(),
            Body::Text(url.clone()),
        ),
    }
}

fn permission_options(
    interaction: &Interaction,
    session_scope: Option<&str>,
    act: Option<&str>,
    reading: &Reading<'_>,
) -> Vec<Opt> {
    let mut options = Vec::new();
    for rung in interaction.rungs() {
        options.push(match rung.answer {
            Answer::AllowOnce => Opt {
                tone: act.map(|_| "bad"),
                ..plain(
                    act.map_or_else(|| "Yes".into(), |act| format!("Yes, {act}")),
                    Does::Send(Answer::AllowOnce),
                )
            },
            Answer::AllowSession { .. } => session(session_scope, rung.answer),
            Answer::Deny { .. } => Opt {
                hint: Some(format!("type to tell {} why", reading.worker)),
                ..plain("No, and tell it why".into(), Does::Deny)
            },
            answer => plain(rung.label, Does::Send(answer)),
        });
    }
    if interaction.answers.contains(&AnswerSpec::AllowChanged) {
        options.push(Opt {
            hint: Some(format!(
                "{} to edit the command",
                crate::theme::glyphs::key_text("⏎")
            )),
            ..plain("Change it first".into(), Does::Change)
        });
    }
    options
}

fn session(rule: Option<&str>, answer: Answer) -> Opt {
    let words = scope::words(rule.unwrap_or_default());
    let label = match words.lead {
        Some(lead) => format!("Yes, allow {lead}"),
        None => "Yes, allow".into(),
    };
    Opt {
        scope: Some(words.code),
        tail: " for this session",
        ..plain(label, Does::Send(answer))
    }
}

fn plain(label: String, does: Does) -> Opt {
    Opt {
        label,
        scope: None,
        tail: "",
        tone: None,
        hint: None,
        does,
    }
}

fn question(verb: &str, target: &str) -> String {
    let mut chars = verb.chars();
    let verb: String = chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    match target {
        "" => format!("{verb}?"),
        target => format!("{verb} {target}?"),
    }
}

/// The last `n` parts of a path, which is what tells two files apart.
fn tail(path: &str, n: usize) -> String {
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    parts[parts.len().saturating_sub(n)..].join("/")
}

fn counts(unified: &str) -> (usize, usize) {
    unified.lines().fold((0, 0), |(added, removed), line| {
        if line.starts_with("+++") || line.starts_with("---") {
            (added, removed)
        } else if line.starts_with('+') {
            (added + 1, removed)
        } else if line.starts_with('-') {
            (added, removed + 1)
        } else {
            (added, removed)
        }
    })
}

fn homed(cwd: &str, home: Option<&str>) -> String {
    home.and_then(|home| cwd.strip_prefix(home))
        .filter(|rest| rest.is_empty() || rest.starts_with(['/', '\\']))
        .map_or_else(|| cwd.to_owned(), |rest| format!("~{rest}"))
}

fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::Caution;

    fn reading() -> Reading<'static> {
        Reading {
            worker: "bingo",
            cwd: "/home/me/bingo",
            home: Some("/home/me"),
            verb: "edit",
            target: "crates/bingo-core/src/session/queue.rs",
        }
    }

    fn labels(ask: &Ask) -> Vec<String> {
        ask.options
            .iter()
            .map(|o| match &o.scope {
                Some(scope) => format!("{} {scope}{}", o.label, o.tail),
                None => o.label.clone(),
            })
            .collect()
    }

    fn bash(caution: Option<Caution>) -> Interaction {
        let mut open = crate::test_support::permission(
            caution.is_none().then_some("Bash(git push:*)"),
            Some(Preview::Command {
                command: "git push --force origin dev".into(),
                cwd: "/home/me/bingo".into(),
                description: None,
                stakes: None,
            }),
        );
        open.answers.push(AnswerSpec::AllowChanged);
        if let InteractionKind::Permission { caution: slot, .. } = &mut open.kind {
            *slot = caution;
        }
        open
    }

    #[test]
    fn a_diff_asks_by_verb_and_path_with_its_counts() {
        let open = crate::test_support::permission(
            Some("Edit(/home/me/bingo/crates/bingo-core/src/session/)"),
            Some(Preview::Diff {
                unified: "--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n context\n".into(),
            }),
        );
        let ask = ask(&open, &reading()).unwrap();
        assert_eq!(ask.title, "Edit session/queue.rs?");
        let right: String = ask.right.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(right, format!("+1 {}1", crate::theme::glyphs::get("minus")));
        assert_eq!(
            labels(&ask),
            [
                "Yes",
                "Yes, allow edits in src/session/ for this session",
                "No, and tell it why"
            ]
        );
        assert_eq!(
            ask.options[2].hint.as_deref(),
            Some("type to tell bingo why")
        );
        assert_eq!(ask.start, 0);
    }

    #[test]
    fn a_command_names_its_directory_from_home_and_offers_a_change() {
        let ask = ask(&bash(None), &reading()).unwrap();
        assert_eq!(ask.title, "Run this command?");
        assert_eq!(ask.right, [("in ~/bingo".to_owned(), "dim")]);
        assert_eq!(
            labels(&ask),
            [
                "Yes",
                "Yes, allow git push for this session",
                "No, and tell it why",
                "Change it first"
            ]
        );
        assert!(!ask.danger);
    }

    #[test]
    fn a_caution_starts_on_no_and_offers_no_session_rung() {
        let ask = ask(
            &bash(Some(Caution {
                reason: "rewrites remote history".into(),
                act: Some("force-push".into()),
            })),
            &reading(),
        )
        .unwrap();
        assert!(ask.danger);
        assert_eq!(
            labels(&ask),
            ["Yes, force-push", "No, and tell it why", "Change it first"]
        );
        assert_eq!(ask.options[0].tone, Some("bad"));
        assert_eq!(ask.start, 1);
        assert_eq!(ask.right, [("rewrites remote history".to_owned(), "bad")]);
    }

    #[test]
    fn an_address_asks_to_fetch_its_host_and_anything_else_reads_its_summary() {
        let url = crate::test_support::permission(
            None,
            Some(Preview::Url {
                url: "https://docs.rs/serde?x=1".into(),
            }),
        );
        assert_eq!(ask(&url, &reading()).unwrap().title, "Fetch docs.rs?");
        let bare = ask(&crate::test_support::permission(None, None), &reading()).unwrap();
        assert_eq!(bare.title, "Edit?");
        assert_eq!(bare.body, Body::Text("Edit src/lib.rs".into()));
    }
}
