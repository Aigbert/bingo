//! The open interaction, answered. Options come from `interaction.answers` —
//! exactly what the kernel will accept — so a row can never promise something
//! the kernel would refuse.
//!
//! Answering is a write and being told is a frame: between the two the
//! interaction is still in `state.interactions`, so the dialog stays on screen
//! with a `…` marker until `InteractionResolved` removes it.

use bingo_sdk::component::{Step, StepState};
use bingo_sdk::{
    Activation, Answer, AnswerSpec, Interaction, InteractionId, InteractionKind, LoginFlow,
    Preview, Question,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::{Line, Span};

use crate::clock::Now;
use crate::composer::Composer;
use crate::effect::Effect;
use crate::form::{self, Form};
use crate::{layers, preview, theme};

pub use crate::ask::options::{Choice, Opt};

/// The surface's own state for the interaction on top of the stack.
#[derive(Clone, Debug, Default)]
pub struct Dialog {
    /// Whose state this is; a different interaction resets everything.
    current: Option<InteractionId>,
    pub focus: usize,
    pub aside: bool,
    pub changing: Option<serde_json::Value>,
    pub expanded: bool,
    /// The feedback or free-text row, open once the answer needs words.
    pub words: Option<Composer>,
    pub chosen: Vec<String>,
    /// Answered, waiting for the frame that closes it.
    pub answered: bool,
    /// A form's own state: which question is on screen, and what each of the
    /// others was left on. `focus` is still the cursor of the one on screen,
    /// so every key, click and window here works on a form unchanged.
    pub form: Form,
}

impl Dialog {
    pub fn current(&self) -> Option<&InteractionId> {
        self.current.as_ref()
    }

    /// Point the dialog at whatever is on top now, forgetting the last one.
    pub fn focus_on(&mut self, interaction: Option<&Interaction>) {
        let id = interaction.map(|i| i.id.clone());
        if id != self.current {
            *self = Self {
                current: id,
                focus: interaction.map_or(0, crate::ask::options::initial),
                words: interaction
                    .filter(|i| {
                        matches!(
                            i.kind,
                            InteractionKind::Login {
                                flow: LoginFlow::Paste,
                                ..
                            }
                        )
                    })
                    .map(|_| Composer::default()),
                ..Self::default()
            };
        }
    }

    pub fn on_key(&mut self, interaction: &Interaction, key: KeyEvent, now: Now) -> Vec<Effect> {
        if key.code != KeyCode::Esc && (self.answered || guarded(interaction, now)) {
            return Vec::new();
        }
        if matches!(key.code, KeyCode::Char('e' | 'o'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.expanded = !self.expanded;
            return Vec::new();
        }
        if key == KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
            && self.words.take().is_some()
        {
            self.changing = None;
            return Vec::new();
        }
        if key.code == KeyCode::Esc {
            return self.send(interaction, cancel(interaction));
        }
        if let InteractionKind::Form { questions, .. } = &interaction.kind {
            let answer = form::on_key(&mut self.form, &mut self.focus, questions, key);
            return self.send(interaction, answer);
        }
        if self.words.is_some() {
            let done = self
                .words
                .as_mut()
                .is_some_and(|words| words_key(words, key));
            return self.typing(interaction, done);
        }
        self.choosing(interaction, key)
    }

    fn typing(&mut self, interaction: &Interaction, done: bool) -> Vec<Effect> {
        if !done {
            return Vec::new();
        }
        let text = self
            .words
            .take()
            .map(|w| w.text().to_string())
            .unwrap_or_default();
        let answer = if let Some(mut input) = self.changing.take() {
            let field = if input["command"].is_string() {
                "command"
            } else {
                "new_string"
            };
            input[field] = serde_json::Value::String(text);
            Answer::AllowChanged { input }
        } else {
            words_answer(interaction, text)
        };
        self.send(interaction, Some(answer))
    }

    fn choosing(&mut self, interaction: &Interaction, key: KeyEvent) -> Vec<Effect> {
        let options = options(interaction);
        match key.code {
            KeyCode::Up => self.focus = self.focus.saturating_sub(1),
            KeyCode::Down => self.focus = (self.focus + 1).min(options.len().saturating_sub(1)),
            KeyCode::Char(' ') => return self.toggle(&options),
            KeyCode::Enter => return self.confirm(interaction, &options),
            KeyCode::Char(c @ '1'..='9') if typed(key) => {
                let Some(index) = digit(c, options.len()) else {
                    return Vec::new();
                };
                self.focus = index;
                return self.confirm(interaction, &options);
            }
            KeyCode::Char(c) if typed(key) => {
                let Some(index) = letter(c, &options) else {
                    return Vec::new();
                };
                self.focus = index;
                return self.confirm(interaction, &options);
            }
            _ => {}
        }
        Vec::new()
    }

    fn toggle(&mut self, options: &[Opt]) -> Vec<Effect> {
        let Some(Choice::Toggle(id)) = options.get(self.focus).map(|o| &o.choice) else {
            return Vec::new();
        };
        match self.chosen.iter().position(|c| c == id) {
            Some(i) => {
                self.chosen.remove(i);
            }
            None => self.chosen.push(id.clone()),
        }
        Vec::new()
    }

    /// Enter on the focused row. In a multiple-choice question every row but
    /// the free-text one confirms the whole set.
    fn confirm(&mut self, interaction: &Interaction, options: &[Opt]) -> Vec<Effect> {
        match options.get(self.focus).map(|o| o.choice.clone()) {
            Some(Choice::Send(answer)) => self.send(interaction, Some(answer)),
            Some(Choice::Change) => Vec::new(),
            Some(Choice::Words) => {
                self.words = Some(Composer::default());
                Vec::new()
            }
            Some(Choice::Toggle(_)) => self.send(
                interaction,
                Some(Answer::Choice {
                    ids: self.chosen.clone(),
                    other: None,
                }),
            ),
            None => Vec::new(),
        }
    }

    fn send(&mut self, interaction: &Interaction, answer: Option<Answer>) -> Vec<Effect> {
        let Some(answer) = answer else {
            return Vec::new();
        };
        self.answered = true;
        vec![Effect::Answer {
            interaction: interaction.id.clone(),
            answer,
            activation: Activation::Keyboard,
        }]
    }
}

/// Keyboard answers before the guard are refused by the kernel, so the dialog
/// does not send them.
fn guarded(interaction: &Interaction, now: Now) -> bool {
    interaction
        .guard_until
        .is_some_and(|until| now.wall < until)
}

/// Edit the words row; `true` when the person is done with it.
fn words_key(words: &mut Composer, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => return true,
        KeyCode::Backspace => words.backspace(),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => words.set(""),
        KeyCode::Left => words.left(),
        KeyCode::Right => words.right(),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            words.insert(&c.to_string())
        }
        _ => {}
    }
    false
}

fn words_answer(interaction: &Interaction, text: String) -> Answer {
    let text = text.trim().to_string();
    match interaction.kind {
        InteractionKind::Permission { .. } => Answer::Deny {
            feedback: (!text.is_empty()).then_some(text),
        },
        _ => Answer::Text { text },
    }
}

/// Leaving a permission prompt is the refusal; leaving anything else cancels.
pub(crate) fn cancel(interaction: &Interaction) -> Option<Answer> {
    if interaction.answers.contains(&AnswerSpec::Deny) {
        return Some(Answer::Deny { feedback: None });
    }
    interaction
        .answers
        .contains(&AnswerSpec::Cancel)
        .then_some(Answer::Cancel)
}

/// A bare letter or digit, not a chord.
fn typed(key: KeyEvent) -> bool {
    (key.modifiers - KeyModifiers::SHIFT).is_empty()
}

fn digit(c: char, len: usize) -> Option<usize> {
    let index = (c as usize) - ('1' as usize);
    (index < len).then_some(index)
}

/// `y`, `a` and `n` name the three permission answers wherever they sit.
fn letter(c: char, options: &[Opt]) -> Option<usize> {
    let wanted = match c {
        'y' => AnswerSpec::AllowOnce,
        'a' => AnswerSpec::AllowSession,
        'n' => AnswerSpec::Deny,
        _ => return None,
    };
    options.iter().position(|o| match &o.choice {
        Choice::Send(answer) => answer.spec() == wanted,
        Choice::Words => wanted == AnswerSpec::Deny,
        Choice::Toggle(_) | Choice::Change => false,
    })
}

/// What this interaction is drawn as. Every card is a box but the form, which
/// is a band — the user's word on 2026-09-04, recorded in design §2 and §10.
pub fn shape(interaction: &Interaction) -> layers::Shape {
    match interaction.kind {
        InteractionKind::Form { .. } => layers::Shape::Band,
        _ => layers::Shape::Boxed,
    }
}

pub use crate::ask::options::options;

/// The card, as the lines inside it (design §4): a bold title, what it is
/// about on its own tints, the one line that asks, then the answers. The box
/// around it, the dimming behind it and its reveal are the frame's.
///
/// `width` bounds nothing here; the caller wraps. `agent` names the
/// sub-session that asked, when it was not the one on screen (ADR-0010 §3);
/// `cwd` is that session's directory, which is what makes a path short.
///
/// Every row comes with the option it belongs to, so one walk lays the card
/// out and a click lands where the eye is.
pub fn rows(
    dialog: &Dialog,
    interaction: &Interaction,
    agent: Option<&str>,
    cwd: &str,
    width: usize,
    room: usize,
    tools: &crate::tool_rows::Looks,
) -> Vec<(Line<'static>, Option<usize>)> {
    if let InteractionKind::Form { questions, title } = &interaction.kind {
        let mut out = form::rows(
            &dialog.form,
            dialog.focus,
            questions,
            form::Head {
                title: title.as_deref(),
                agent,
                width,
                room,
            },
        );
        out.extend(answering(dialog));
        return out;
    }
    let mut out = vec![(title(interaction, agent, cwd, width, tools), None)];
    let mut preview = body(dialog, interaction);
    let limit = room.saturating_sub(options(interaction).len() + 5).max(3);
    if preview.len() > limit {
        preview.truncate(limit.saturating_sub(1));
        preview.push(Line::from(Span::styled(
            "     ╰ … ctrl+o whole diff",
            theme::dim(),
        )));
    }
    out.extend(preview.into_iter().map(|line| (line, None)));
    if let Some(question) = question(interaction, cwd) {
        out.push((Line::default(), None));
        out.push((question, None));
    }
    for (index, option) in options(interaction).iter().enumerate() {
        out.extend(
            option_lines(
                dialog,
                index,
                option,
                matches!(interaction.kind, InteractionKind::Login { .. }),
            )
            .into_iter()
            .map(|line| (line, Some(index))),
        );
    }
    out.extend(caution_rows(interaction));
    out.extend(answering(dialog));
    out
}

fn caution_rows(interaction: &Interaction) -> Vec<(Line<'static>, Option<usize>)> {
    let mut out = Vec::new();
    if let InteractionKind::Permission {
        caution: Some(caution),
        ..
    } = &interaction.kind
    {
        out.push((
            Line::from(Span::styled(
                format!("     no allow this session here: {}", caution.reason),
                theme::presence(),
            )),
            None,
        ));
    }
    out
}

/// The mark of an answer already sent: it stays on screen until the frame that
/// closes the card arrives.
fn answering(dialog: &Dialog) -> Vec<(Line<'static>, Option<usize>)> {
    if !dialog.answered {
        return Vec::new();
    }
    vec![(
        Line::from(Span::styled(
            format!("  {} waiting for the kernel", theme::ellipsis()),
            theme::dim(),
        )),
        None,
    )]
}

/// What kind of card this is, in the words a person reads first. A
/// permission is the row of the step that asks, which is where it opened; a
/// form draws its own head, which is the tab row (see [`form::rows`]).
fn title(
    interaction: &Interaction,
    agent: Option<&str>,
    cwd: &str,
    width: usize,
    tools: &crate::tool_rows::Looks,
) -> Line<'static> {
    if let InteractionKind::Permission { tool, input, .. } = &interaction.kind {
        return crate::transcript::asked(&asking(tool, input, agent, cwd, tools), width);
    }
    Line::from(form::head(&named(interaction), agent))
}

/// The step a permission asks about, in its row's words.
fn asking(
    tool: &str,
    input: &serde_json::Value,
    agent: Option<&str>,
    cwd: &str,
    tools: &crate::tool_rows::Looks,
) -> Step {
    let mut step = crate::tool_rows::resolve(tool, input, tools, theme::bullet().is_ascii());
    let target = shorten(step.target.lines().next().unwrap_or_default(), cwd);
    step.target = match agent {
        Some(name) if target.is_empty() => format!("· {name}"),
        Some(name) => format!("{target} · {name}"),
        None => target,
    };
    step.state = StepState::Asked;
    step
}

fn named(interaction: &Interaction) -> String {
    match &interaction.kind {
        InteractionKind::Permission { tool, .. } => tool.clone(),
        InteractionKind::Question(Question { header, .. }) => {
            header.clone().unwrap_or_else(|| "Question".to_string())
        }
        InteractionKind::Confirm { title, .. } => title.clone(),
        InteractionKind::Login { provider, .. } => format!("Sign in to {provider}"),
        InteractionKind::Form { .. } => "Questions".to_string(),
    }
}

/// The one line that asks. A permission asks about the call the kernel
/// summarised; a question and a confirmation carry their own words.
fn question(interaction: &Interaction, cwd: &str) -> Option<Line<'static>> {
    let text = match &interaction.kind {
        InteractionKind::Permission {
            summary, preview, ..
        } => match preview.as_deref() {
            Some(Preview::Command { .. }) => "Do you want to run this command?".into(),
            Some(Preview::Url { .. }) => "Do you want to fetch this page?".into(),
            Some(Preview::Diff { .. }) => format!("Do you want to {}?", shorten(summary, cwd)),
            None => format!("Do you want to allow {}?", shorten(summary, cwd)),
        },
        InteractionKind::Question(Question { question, .. }) => question.clone(),
        InteractionKind::Form { .. } => return None,
        InteractionKind::Confirm { detail, .. } => detail.clone(),
        InteractionKind::Login { .. } => return None,
    };
    Some(Line::from(Span::styled(
        format!("     {text}"),
        theme::text(),
    )))
}

fn shorten(text: &str, cwd: &str) -> String {
    crate::paths::shorten_in(text, cwd, crate::paths::home())
}

fn body(dialog: &Dialog, interaction: &Interaction) -> Vec<Line<'static>> {
    match &interaction.kind {
        InteractionKind::Permission {
            preview: Some(preview),
            ..
        } => preview_lines(preview, dialog.expanded),
        InteractionKind::Login { flow, .. } => login_lines(flow),
        _ => Vec::new(),
    }
}

/// The diff or the command, on the tints of what it does.
fn preview_lines(preview: &Preview, expanded: bool) -> Vec<Line<'static>> {
    let (rows, hidden) = preview::lines(preview, expanded);
    let mut out = vec![Line::from(Span::styled(
        format!("     {}", theme::glyphs().border.top_left),
        theme::dim(),
    ))];
    out.extend(rows.into_iter().map(|line| {
        let mut spans = vec![Span::styled(
            format!("     {} ", theme::glyphs().border.vertical_left),
            theme::dim(),
        )];
        spans.extend(line.spans);
        Line::from(spans)
    }));
    let tail = if hidden > 0 {
        format!("     ╰ … +{hidden} lines")
    } else {
        "     ╰".into()
    };
    out.push(Line::from(Span::styled(tail, theme::dim())));
    out
}

fn login_lines(flow: &LoginFlow) -> Vec<Line<'static>> {
    match flow {
        LoginFlow::Browser { url } => vec![
            indented("Finish in the browser. If it did not open, go to:"),
            link(url),
        ],
        LoginFlow::Device { url, code } => vec![
            indented("Open this address and enter the code:"),
            link(url),
            indented(&format!("code: {code}")),
        ],
        LoginFlow::Paste => vec![indented("A credential minted elsewhere.")],
    }
}

fn option_lines(dialog: &Dialog, index: usize, option: &Opt, secret: bool) -> Vec<Line<'static>> {
    let focused = index == dialog.focus;
    let style = if focused { theme::text() } else { theme::dim() };
    let label = if let Some(words) = dialog.words.as_ref().filter(|_| focused) {
        let prefix = if dialog.changing.is_some() || !option.label.starts_with("No,") {
            ""
        } else {
            "No, because "
        };
        let text = if secret {
            concealed(words.text())
        } else {
            words.text().to_owned()
        };
        format!("{prefix}{text}{}", theme::caret())
    } else {
        format!("{}{}", box_mark(dialog, option), option.label)
    };
    let cursor = if focused && !dialog.aside {
        format!("   {} ", theme::cursor())
    } else {
        "     ".into()
    };
    let mut out = vec![Line::from(vec![
        Span::styled(cursor, theme::presence()),
        Span::styled(format!("{}  {label}", index + 1), style),
    ])];
    if let Some(description) = &option.description {
        out.push(Line::from(Span::styled(
            format!("        {description}"),
            theme::dim(),
        )));
    }
    out
}

/// A secret shows that something was pasted, never how long it is.
fn concealed(text: &str) -> String {
    if text.trim().is_empty() {
        String::new()
    } else {
        "••••••••  pasted".into()
    }
}

/// One of a set is a box that is ticked or not; one of a choice is neither.
fn box_mark(dialog: &Dialog, option: &Opt) -> String {
    match &option.choice {
        Choice::Toggle(id) => format!("{} ", theme::todo(dialog.chosen.iter().any(|c| c == id))),
        _ => String::new(),
    }
}

fn indented(text: &str) -> Line<'static> {
    Line::from(Span::styled(format!("  {text}"), theme::text()))
}

fn link(url: &str) -> Line<'static> {
    Line::from(Span::styled(format!("  {url}"), theme::link()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_steps_keep_the_declared_look_and_target() {
        for (name, input, look, _, target) in crate::tool_rows::cases() {
            let tools = crate::tool_rows::declared(name, look.clone());
            let step = asking(name, &input, None, "/work", &tools);
            assert_eq!(step.target, target);
            assert_eq!(step.verb, look.verb);
            assert_eq!(step.mark, look.mark);
            assert_eq!(step.reads, look.target.map(|t| t.reads));
            assert_eq!(step.tier, bingo_sdk::component::Tier::Lookup);
            assert_eq!(step.state, StepState::Asked);
        }
    }

    #[test]
    fn rough_edges_questions_are_sentences_for_the_preview() {
        for (preview, expected) in [
            (Some(serde_json::from_value(serde_json::json!({"kind":"command","command":"echo hi","cwd":"/work","description":"Print a greeting"})).unwrap()), "Do you want to run this command?"),
            (Some(Preview::Url { url: "https://example.test".into() }), "Do you want to fetch this page?"),
            (None, "Do you want to allow Edit src/lib.rs?"),
        ] {
            let interaction = crate::test_support::permission(None, preview);
            assert_eq!(question(&interaction, "/work").unwrap().to_string().trim(), expected);
        }
    }

    #[test]
    fn a_pasted_secret_draws_one_fixed_line_whatever_its_length() {
        let option = Opt {
            label: "paste".into(),
            description: None,
            choice: Choice::Words,
        };
        let drawn = |text: &str| {
            let mut words = crate::composer::Composer::default();
            words.insert(text);
            let dialog = Dialog {
                words: Some(words),
                ..Dialog::default()
            };
            option_lines(&dialog, 0, &option, true)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };
        let short = drawn("sk-dummy");
        let huge = drawn(&format!("sk-dummy-{}", "x".repeat(4096)));
        assert_eq!(short, huge);
        assert_eq!(huge.len(), 1);
        assert!(huge[0].contains("••••••••  pasted"), "{huge:?}");
        assert!(!huge[0].contains("•••••••••") && !huge[0].contains("sk-"));
    }
}
