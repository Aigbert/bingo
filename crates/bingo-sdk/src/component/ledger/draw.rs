use super::{Marks, Step, StepState, Tier};
use crate::{
    Reads, Upshot, View,
    view::{TextChild, TextMotion, TextStyle},
};
use unicode_width::UnicodeWidthStr;

type Segments = Vec<(String, &'static str)>;

impl Step {
    /// The fixed target column, extended for verbs wider than six cells.
    pub fn target_column(&self) -> usize {
        13.max(7 + self.verb.width())
    }

    /// A row in the shared view vocabulary; motion belongs to the surface clock.
    pub fn drawing(&self, marks: &Marks, width: usize) -> View {
        let ascii = marks.asked.is_ascii();
        let right = self.right(ascii);
        let right_width: usize = right.iter().map(|(text, _)| text.width()).sum();
        let ax = self.target_column();
        let available =
            width.saturating_sub(ax + 2 + if right.is_empty() { 0 } else { right_width + 2 });
        let mut children = vec![TextChild::String("   ".into()), self.mark_node(marks)];
        children.push(TextChild::String(" ".into()));
        children.push(node(&self.verb, self.verb_ink(), None));
        children.push(TextChild::String(
            " ".repeat(ax.saturating_sub(5 + self.verb.width())),
        ));
        let target = self.target_segments(available, ascii);
        let target_width: usize = target.iter().map(|(text, _)| text.width()).sum();
        children.extend(target.into_iter().map(|(text, ink)| node(&text, ink, None)));
        if !right.is_empty() {
            children.push(TextChild::String(
                " ".repeat(width.saturating_sub(2 + right_width + ax + target_width)),
            ));
            children.extend(right.into_iter().map(|(text, ink)| node(&text, ink, None)));
        }
        View::Text {
            children,
            style: TextStyle {
                wrap: Some("truncate".into()),
                ..Default::default()
            },
        }
    }

    fn quiet(&self) -> bool {
        matches!(self.state, StepState::Stopped | StepState::Skipped)
    }

    fn mark_node(&self, marks: &Marks) -> TextChild {
        let (glyph, ink, motion) = match self.state {
            StepState::Running => (self.mark.to_string(), "glow", Some(TextMotion::Spin)),
            StepState::Asked => (
                marks.asked.clone(),
                "amberGlow",
                Some(TextMotion::Breathe {
                    with: "you".into(),
                    period_ms: 1100,
                    depth: 0.65,
                }),
            ),
            _ => {
                let ink = if self.state == StepState::Failed {
                    "bad"
                } else if self.quiet() {
                    "faint"
                } else if self.tier == Tier::Lookup {
                    "muted"
                } else {
                    "fg"
                };
                let motion = self.landed_at.map(|at_ms| TextMotion::Fade {
                    curve: Some(crate::view::FadeCurve::EaseOut),
                    from: "glow".into(),
                    at_ms,
                    ms: 420,
                });
                (self.mark.to_string(), ink, motion)
            }
        };
        node(&glyph, ink, motion)
    }

    fn verb_ink(&self) -> &'static str {
        if self.state == StepState::Failed {
            "bad"
        } else if self.quiet() || self.tier == Tier::Lookup {
            "dim"
        } else {
            "muted"
        }
    }

    fn target_segments(&self, width: usize, ascii: bool) -> Segments {
        let ink = if self.quiet() {
            "dim"
        } else if self.tier == Tier::Lookup {
            "muted"
        } else {
            "fg"
        };
        if self.reads == Some(Reads::Path) {
            let (head, base) =
                super::super::row::short_path(&self.target, width, if ascii { "..." } else { "…" });
            return super::super::row::cut(
                vec![(head, "dim"), (base, ink)],
                width,
                if ascii { "..." } else { "…" },
                "dim",
                &|s| s.width(),
            );
        }
        let ink = if self.reads == Some(Reads::Command) && !self.quiet() {
            "code"
        } else {
            ink
        };
        super::super::row::cut(
            vec![(self.target.clone(), ink)],
            width,
            if ascii { "..." } else { "…" },
            "dim",
            &|s| s.width(),
        )
    }

    fn right(&self, ascii: bool) -> Segments {
        let mut out = match self.state {
            StepState::Running => self
                .elapsed_ms
                .filter(|ms| *ms >= 1000)
                .map(|ms| vec![(super::super::row::fmt_dur(ms), "dim")])
                .unwrap_or_default(),
            StepState::Asked => vec![("needs you".into(), "amber")],
            StepState::Stopped => vec![("stopped".into(), "muted")],
            StepState::Skipped => vec![("said no".into(), "muted")],
            _ => self.result(ascii),
        };
        if !matches!(self.state, StepState::Running | StepState::Asked) {
            let answer = match &self.answer {
                Some(Upshot::Words { text, .. }) => Some(text),
                _ => None,
            };
            for text in answer.into_iter().chain(self.why.as_ref()) {
                if !out.is_empty() {
                    out.push((if ascii { " . " } else { " · " }.into(), "faint"));
                }
                out.push((text.clone(), "dim"));
            }
        }
        out
    }

    fn result(&self, ascii: bool) -> Segments {
        match &self.upshot {
            Some(Upshot::Diff { added, removed }) => {
                let mut out = Vec::new();
                if *added > 0 {
                    out.push((format!("+{added}"), "good"));
                }
                if *removed > 0 {
                    if !out.is_empty() {
                        out.push((" ".into(), "fg"));
                    }
                    out.push((format!("{}{removed}", if ascii { "-" } else { "−" }), "bad"));
                }
                out
            }
            Some(Upshot::Words { text, .. }) => vec![(
                text.clone(),
                if self.state == StepState::Failed {
                    "bad"
                } else if self.tier == Tier::Lookup {
                    "dim"
                } else {
                    "muted"
                },
            )],
            None if self.state == StepState::Failed => vec![("failed".into(), "bad")],
            None => Vec::new(),
        }
    }
}

fn node(text: &str, ink: &str, motion: Option<TextMotion>) -> TextChild {
    TextChild::Node(Box::new(View::Text {
        children: vec![TextChild::String(text.into())],
        style: TextStyle {
            color: Some(ink.into()),
            motion: motion.map(Box::new),
            ..Default::default()
        },
    }))
}

#[cfg(test)]
mod tests;
