//! The open interaction, answered. Options come from `interaction.answers` —
//! exactly what the kernel will accept — so a row can never promise something
//! the kernel would refuse.
//!
//! Answering is a write and being told is a frame: between the two the
//! interaction is still in `state.interactions`, so the dialog stays on screen
//! with a `…` marker until `InteractionResolved` removes it.

use bingo_sdk::{
    Activation, Answer, AnswerSpec, Interaction, InteractionId, InteractionKind, LoginFlow,
    Question,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::{Line, Span};

use crate::clock::Now;
use crate::composer::Composer;
use crate::effect::Effect;
use crate::form::{self, Form};
use crate::{layers, theme};

pub use crate::ask::options::{Choice, Opt};

/// The surface's own state for the interaction on top of the stack.
#[derive(Clone, Debug, Default)]
pub struct Dialog {
    /// Whose state this is; a different interaction resets everything.
    current: Option<InteractionId>,
    pub focus: usize,
    pub aside: bool,
    /// The feedback or free-text row, open once the answer needs words.
    pub words: Option<Composer>,
    pub chosen: Vec<String>,
    /// Answered, waiting for the frame that closes it.
    pub answered: bool,
    /// The question tab the `ask` mod last drew and when it came on, so
    /// the guard restarts on each tab.
    pub tab_at: Option<(usize, jiff::Timestamp)>,
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
        if key == KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
            && self.words.take().is_some()
        {
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
        let text = text.trim().to_string();
        self.send(interaction, Some(Answer::Text { text }))
    }

    fn choosing(&mut self, interaction: &Interaction, key: KeyEvent) -> Vec<Effect> {
        let options = options(interaction);
        match key.code {
            KeyCode::Up => self.focus = self.focus.saturating_sub(1),
            KeyCode::Down => self.focus = (self.focus + 1).min(options.len().saturating_sub(1)),
            KeyCode::Char(' ') => return self.toggle(&options),
            KeyCode::Enter => return self.confirm(interaction, &options),
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
pub(crate) fn words_key(words: &mut Composer, key: KeyEvent) -> bool {
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
/// sub-session that asked, when it was not the one on screen (ADR-0010 §3).
///
/// Every row comes with the option it belongs to, so one walk lays the card
/// out and a click lands where the eye is.
pub fn rows(
    dialog: &Dialog,
    interaction: &Interaction,
    agent: Option<&str>,
    width: usize,
    room: usize,
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
    let mut out = vec![(Line::from(form::head(&named(interaction), agent)), None)];
    out.extend(body(interaction).into_iter().map(|line| (line, None)));
    if let Some(question) = question(interaction) {
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
    out.extend(answering(dialog));
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

fn named(interaction: &Interaction) -> String {
    match &interaction.kind {
        InteractionKind::Permission { .. } => String::new(),
        InteractionKind::Question(Question { header, .. }) => {
            header.clone().unwrap_or_else(|| "Question".to_string())
        }
        InteractionKind::Confirm { title, .. } => title.clone(),
        InteractionKind::Login { provider, .. } => format!("Sign in to {provider}"),
        InteractionKind::Form { .. } => "Questions".to_string(),
    }
}

/// The one line that asks: a question and a confirmation carry their own
/// words. A permission is the dock's (`ask`), never this card's.
fn question(interaction: &Interaction) -> Option<Line<'static>> {
    let text = match &interaction.kind {
        InteractionKind::Question(Question { question, .. }) => question.clone(),
        InteractionKind::Confirm { detail, .. } => detail.clone(),
        InteractionKind::Permission { .. }
        | InteractionKind::Form { .. }
        | InteractionKind::Login { .. } => return None,
    };
    Some(Line::from(Span::styled(
        format!("     {text}"),
        theme::text(),
    )))
}

fn body(interaction: &Interaction) -> Vec<Line<'static>> {
    match &interaction.kind {
        InteractionKind::Login { flow, .. } => login_lines(flow),
        _ => Vec::new(),
    }
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
        let text = if secret {
            concealed(words.text())
        } else {
            words.text().to_owned()
        };
        format!("{text}{}", theme::caret())
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
