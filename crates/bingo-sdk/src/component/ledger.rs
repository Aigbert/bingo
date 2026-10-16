//! A turn's steps: the row each step draws while the turn runs, and the one
//! line a finished turn folds to (ADR-0095 §6–§8). The words and their inks
//! are worked out here, once; the glyph a state is drawn with is the
//! surface's, so each caller passes its own.
use super::row::{Ink, Piece, laid};
use super::*;
use crate::{Upshot, view::Tone};

/// Where a step's row starts: marks sit at col 1, steps under the words.
pub const STEP_INDENT: &str = "   ";

/// What a step is doing, as far as the turn has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Running,
    /// Waits for a person's answer.
    Asked,
    Done,
    Failed,
    /// Was in flight when the person stopped the turn.
    Stopped,
}

/// One tool call of a turn, in the words a row shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// What the step does, one lowercase word: `read`, `bash`.
    pub verb: String,
    /// What it does it to, shortened for one row.
    pub target: String,
    pub state: StepState,
    /// What the tool said the call came to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upshot: Option<Upshot>,
    /// What the person answered when the step asked: `you: yes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<Upshot>,
}

/// How a turn ended, for its folded line. A failed turn ends `done` here:
/// the line under the turn says why it failed, and a failed step already says
/// `failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TurnEnd {
    Done,
    Stopped,
}

/// The `Ledger` site: the rows of a running turn's steps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LedgerProps {
    pub steps: Vec<Step>,
}

/// The `TurnFold` site: a finished turn as one line. The stop line under a
/// stopped turn is the interruption's own row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TurnFoldProps {
    pub steps: Vec<Step>,
    pub outcome: TurnEnd,
    /// From the turn's first item to its last, in milliseconds.
    pub elapsed: u64,
    /// The turn thought before or between its steps.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub thought: bool,
    /// The person opened the fold to read the steps inline.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_expanded: bool,
}

impl Step {
    /// The ink of the step's glyph.
    pub fn ink(&self) -> Ink {
        match self.state {
            StepState::Running => Ink::Presence,
            StepState::Asked => Ink::Glow,
            StepState::Done => Ink::Good,
            StepState::Failed | StepState::Stopped => Ink::Bad,
        }
    }

    /// What the row says at its right end: the tool's upshot, else what the
    /// state says on its own. A running step says nothing yet.
    pub fn outcome(&self) -> Option<Upshot> {
        let said = |text: &str, tone| Upshot::Words {
            text: text.into(),
            tone,
        };
        match self.state {
            StepState::Running => None,
            StepState::Asked => Some(said("needs you", Tone::Attention)),
            StepState::Done => self.upshot.clone(),
            StepState::Failed => self.upshot.clone().or(Some(said("failed", Tone::Bad))),
            StepState::Stopped => self.upshot.clone().or(Some(said("stopped", Tone::Bad))),
        }
    }

    /// The step's row: `   ✓ read src/parse.rs` with the outcome
    /// right-aligned — or, where the person answered the step, the outcome
    /// after the words and the answer at the right.
    pub fn row(&self, glyph: &str, width: usize, cells: impl Fn(&str) -> usize) -> Vec<Piece> {
        let words = match self.target.is_empty() {
            true => self.verb.clone(),
            false => format!("{} {}", self.verb, self.target),
        };
        let words_ink = match self.state {
            StepState::Asked => Ink::Glow,
            _ => Ink::Plain,
        };
        let mut left = vec![
            Piece::new(STEP_INDENT, Ink::Plain),
            Piece::new(format!("{glyph} "), self.ink()),
            Piece::new(words, words_ink),
        ];
        let right = match &self.answer {
            Some(answer) => {
                if let Some(outcome) = self.outcome() {
                    left.push(Piece::new("  ", Ink::Plain));
                    left.extend(pieces(&outcome, Ink::Dim));
                }
                pieces(answer, Ink::Dim)
            }
            None => self
                .outcome()
                .map(|o| pieces(&o, Ink::Dim))
                .unwrap_or_default(),
        };
        laid(left, right, width, cells)
    }
}

/// An upshot as pieces: `+1` good and `−1` bad, words in their tone's ink.
/// Neutral words take `neutral`.
pub fn pieces(upshot: &Upshot, neutral: Ink) -> Vec<Piece> {
    match upshot {
        Upshot::Diff { added, removed } => diff(*added, *removed),
        Upshot::Words { text, tone } => {
            let ink = match tone {
                Tone::Neutral => neutral,
                tone => Ink::of(*tone),
            };
            vec![Piece::new(text.clone(), ink)]
        }
    }
}

fn diff(added: u32, removed: u32) -> Vec<Piece> {
    let mut out = Vec::new();
    if added > 0 {
        out.push(Piece::new(format!("+{added}"), Ink::Good));
    }
    if removed > 0 {
        if !out.is_empty() {
            out.push(Piece::new(" ", Ink::Dim));
        }
        out.push(Piece::new(format!("\u{2212}{removed}"), Ink::Bad));
    }
    out
}

/// Consecutive steps of one verb, read as one: `read ×18`, `edit +212 −88`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub verb: String,
    pub count: usize,
    /// What the run came to: bad news beats a diff, a diff beats good news,
    /// and plain words are left to the expanded steps.
    pub outcome: Option<Upshot>,
}

impl Run {
    fn of(steps: &[Step]) -> Self {
        // A stopped step's own word would repeat the fold's ` · stopped`.
        let said = |s: &Step| match s.state {
            StepState::Stopped => s.upshot.clone(),
            _ => s.outcome(),
        };
        let outcomes: Vec<Upshot> = steps.iter().filter_map(said).collect();
        let worded = |wanted: Tone| {
            outcomes
                .iter()
                .rev()
                .find(|o| matches!(o, Upshot::Words { tone, .. } if *tone == wanted))
        };
        let (mut added, mut removed, mut changed) = (0, 0, false);
        for outcome in &outcomes {
            if let Upshot::Diff {
                added: a,
                removed: r,
            } = outcome
            {
                (added, removed, changed) = (added + a, removed + r, true);
            }
        }
        let summed = changed.then_some(Upshot::Diff { added, removed });
        let outcome = worded(Tone::Bad)
            .cloned()
            .or(summed)
            .or_else(|| worded(Tone::Good).cloned());
        Self {
            verb: steps.first().map(|s| s.verb.clone()).unwrap_or_default(),
            count: steps.len(),
            outcome,
        }
    }

    fn pieces(&self) -> Vec<Piece> {
        let mut label = self.verb.clone();
        if self.count > 1 {
            label.push_str(&format!(" \u{d7}{}", self.count));
        }
        let mut out = vec![Piece::new(label, Ink::Dim)];
        if let Some(outcome) = &self.outcome {
            let words = pieces(outcome, Ink::Dim);
            if !words.is_empty() {
                out.push(Piece::new(" ", Ink::Dim));
                out.extend(words);
            }
        }
        out
    }
}

/// The glyphs a fold draws, from the surface's table.
#[derive(Clone, Copy, Debug)]
pub struct FoldGlyphs<'a> {
    pub folded: &'a str,
    pub open: &'a str,
}

impl TurnFoldProps {
    /// The turn's steps grouped into runs of one verb, in order.
    pub fn runs(&self) -> Vec<Run> {
        self.steps
            .chunk_by(|a, b| a.verb == b.verb)
            .map(Run::of)
            .collect()
    }

    /// The folded line, `▸ 5 steps · read · edit +1 −1 … 7s`; nothing for a
    /// turn that neither stepped nor thought.
    pub fn row(
        &self,
        glyphs: FoldGlyphs,
        width: usize,
        cells: impl Fn(&str) -> usize,
    ) -> Option<Vec<Piece>> {
        let left = self.fold_words(&glyphs)?;
        let right = vec![Piece::new(elapsed(self.elapsed), Ink::Dim)];
        Some(laid(left, right, width, cells))
    }

    /// `▸ 5 steps · read · grep · edit +1 −1`, or `▸ thought` for a turn
    /// that only thought; nothing for a turn that did neither.
    fn fold_words(&self, glyphs: &FoldGlyphs) -> Option<Vec<Piece>> {
        if self.steps.is_empty() && !self.thought {
            return None;
        }
        let glyph = match self.is_expanded {
            true => glyphs.open,
            false => glyphs.folded,
        };
        let mut out = vec![
            Piece::new(STEP_INDENT, Ink::Plain),
            Piece::new(format!("{glyph} "), Ink::Dim),
        ];
        out.push(Piece::new(count(self.steps.len()), Ink::Dim));
        for run in self.runs() {
            out.push(Piece::new(" \u{b7} ", Ink::Dim));
            out.extend(run.pieces());
        }
        if self.outcome == TurnEnd::Stopped {
            out.push(Piece::new(" \u{b7} stopped", Ink::Bad));
        }
        out.extend(self.answers());
        Some(out)
    }

    /// What the person answered along the way, each answer once.
    fn answers(&self) -> Vec<Piece> {
        let mut said: Vec<&str> = Vec::new();
        for answer in self.steps.iter().filter_map(|s| s.answer.as_ref()) {
            if let Upshot::Words { text, .. } = answer
                && !said.contains(&text.as_str())
            {
                said.push(text);
            }
        }
        said.into_iter()
            .map(|text| Piece::new(format!(" \u{b7} {text}"), Ink::Ash))
            .collect()
    }
}

fn count(steps: usize) -> String {
    match steps {
        0 => "thought".into(),
        n => format!("{n} {}", plural(n)),
    }
}

/// The noun `steps` of them take: `1 step`, `3 steps`.
pub fn plural(steps: usize) -> &'static str {
    match steps {
        1 => "step",
        _ => "steps",
    }
}

/// A span of time the way a fold says it: `7s`, `6m 12s`, `2h 5m`; never
/// less than a second.
pub fn elapsed(ms: u64) -> String {
    let secs = (ms / 1000).max(1);
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m {}s", secs / 60, secs % 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(verb: &str, target: &str, state: StepState, upshot: Option<Upshot>) -> Step {
        Step {
            verb: verb.into(),
            target: target.into(),
            state,
            upshot,
            answer: None,
        }
    }
    fn text(pieces: &[Piece]) -> String {
        pieces.iter().map(|p| p.text.as_str()).collect()
    }
    fn cells(s: &str) -> usize {
        s.chars().count()
    }
    const GLYPHS: FoldGlyphs = FoldGlyphs {
        folded: "\u{25b8}",
        open: "\u{25be}",
    };
    fn prototype_turn() -> TurnFoldProps {
        let done = StepState::Done;
        TurnFoldProps {
            steps: vec![
                step("read", "src/main.rs", done, None),
                step("grep", "total", done, Some(Upshot::plain("1 file"))),
                step(
                    "edit",
                    "src/main.rs",
                    done,
                    Some(Upshot::Diff {
                        added: 1,
                        removed: 1,
                    }),
                ),
                step("test", "", done, Some(Upshot::good("3 passed"))),
                step("run", "", done, None),
            ],
            outcome: TurnEnd::Done,
            elapsed: 7_400,
            thought: false,
            is_expanded: false,
        }
    }

    #[test]
    fn a_finished_turn_folds_to_the_prototype_line() {
        let row = prototype_turn().row(GLYPHS, 80, cells).unwrap();
        let line = text(&row);
        assert!(
            line.starts_with("   \u{25b8} 5 steps \u{b7} read \u{b7} grep \u{b7} edit +1 \u{2212}1 \u{b7} test 3 passed \u{b7} run "),
            "{line:?}"
        );
        assert!(line.ends_with(" 7s"), "{line:?}");
        assert_eq!(cells(&line), 78);
        let inks: Vec<(String, Ink)> = row.iter().map(|p| (p.text.clone(), p.ink)).collect();
        assert!(inks.contains(&("+1".into(), Ink::Good)));
        assert!(inks.contains(&("\u{2212}1".into(), Ink::Bad)));
        assert!(inks.contains(&("3 passed".into(), Ink::Good)));
    }

    #[test]
    fn long_runs_count_and_sum() {
        let mut steps = vec![step("read", "a", StepState::Done, None); 18];
        let edit = Some(Upshot::Diff {
            added: 200,
            removed: 80,
        });
        steps.push(step("edit", "a", StepState::Done, edit));
        let edit = Some(Upshot::Diff {
            added: 12,
            removed: 8,
        });
        steps.push(step("edit", "b", StepState::Done, edit));
        let fold = TurnFoldProps {
            steps,
            outcome: TurnEnd::Done,
            elapsed: 372_000,
            thought: false,
            is_expanded: true,
        };
        let line = text(&fold.row(GLYPHS, 80, cells).unwrap());
        assert!(
            line.starts_with(
                "   \u{25be} 20 steps \u{b7} read \u{d7}18 \u{b7} edit \u{d7}2 +212 \u{2212}88"
            ),
            "{line:?}"
        );
        assert!(line.ends_with(" 6m 12s"), "{line:?}");
    }

    #[test]
    fn a_stopped_turn_says_so() {
        let steps = vec![
            step("read", "a", StepState::Done, None),
            step("grep", "b", StepState::Done, None),
            step("bash", "cargo test", StepState::Stopped, None),
        ];
        let fold = TurnFoldProps {
            steps: steps.clone(),
            outcome: TurnEnd::Stopped,
            elapsed: 900,
            thought: true,
            is_expanded: false,
        };
        let line = text(&fold.row(GLYPHS, 100, cells).unwrap());
        assert!(line.contains("grep \u{b7} bash \u{b7} stopped"), "{line:?}");
        assert!(line.ends_with(" 1s"));
        let nothing = TurnFoldProps {
            steps: vec![],
            thought: false,
            ..fold
        };
        assert_eq!(
            nothing.row(GLYPHS, 100, cells),
            None,
            "no fold line for a turn with nothing to fold"
        );
    }

    #[test]
    fn a_failed_turn_says_failed_once_on_the_step_that_failed() {
        let fold = TurnFoldProps {
            steps: vec![step("bash", "cargo test", StepState::Failed, None)],
            outcome: TurnEnd::Done,
            elapsed: 1_000,
            thought: false,
            is_expanded: false,
        };
        let line = text(&fold.row(GLYPHS, 80, cells).unwrap());
        assert!(
            line.starts_with("   \u{25b8} 1 step \u{b7} bash failed  "),
            "{line:?}"
        );
        assert_eq!(line.matches("failed").count(), 1, "{line:?}");
    }

    #[test]
    fn step_rows_say_their_state_at_the_right() {
        let asked = step("bash", "cargo test -q parse", StepState::Asked, None);
        let row = asked.row("\u{25c6}", 60, cells);
        assert!(text(&row).starts_with("   \u{25c6} bash cargo test -q parse "));
        assert!(text(&row).ends_with(" needs you"));
        assert_eq!(cells(&text(&row)), 58);
        assert_eq!(row.last().map(|p| p.ink), Some(Ink::Glow));
        let running = step("bash", "cargo test", StepState::Running, None);
        assert_eq!(
            text(&running.row("\u{25cc}", 60, cells)),
            "   \u{25cc} bash cargo test"
        );
        let failed = step("bash", "x", StepState::Failed, Some(Upshot::bad("exit 1")));
        assert!(text(&failed.row("\u{2717}", 60, cells)).ends_with(" exit 1"));
        let found = step("grep", "x", StepState::Done, Some(Upshot::plain("2 files")));
        let row = found.row("\u{2713}", 60, cells);
        assert_eq!(row.last().map(|p| p.ink), Some(Ink::Dim));
    }

    #[test]
    fn an_answered_step_says_the_answer_at_the_right_and_the_fold_repeats_it_once() {
        let mut edit = step(
            "edit",
            "src/main.rs",
            StepState::Done,
            Some(Upshot::Diff {
                added: 1,
                removed: 1,
            }),
        );
        edit.answer = Some(Upshot::plain("you: yes"));
        let row = text(&edit.row("\u{2713}", 60, cells));
        assert!(
            row.starts_with("   \u{2713} edit src/main.rs  +1 \u{2212}1 "),
            "{row:?}"
        );
        assert!(row.ends_with(" you: yes"), "{row:?}");
        let mut stopped = step("bash", "sleep 60", StepState::Stopped, None);
        stopped.answer = edit.answer.clone();
        let row = text(&stopped.row("\u{2717}", 60, cells));
        assert!(
            row.contains("sleep 60  stopped "),
            "the outcome stays: {row:?}"
        );
        let mut refused = step("bash", "rm -rf build", StepState::Done, None);
        refused.answer = Some(Upshot::Words {
            text: "you: no".into(),
            tone: Tone::Bad,
        });
        let fold = TurnFoldProps {
            steps: vec![
                step("read", "a", StepState::Done, None),
                edit,
                refused.clone(),
                refused,
            ],
            outcome: TurnEnd::Done,
            elapsed: 1_000,
            thought: false,
            is_expanded: false,
        };
        let pieces = fold.row(GLYPHS, 80, cells).unwrap();
        let line = text(&pieces);
        assert!(
            line.contains("bash \u{d7}2 \u{b7} you: yes \u{b7} you: no "),
            "each answer once: {line:?}"
        );
        assert!(pieces.iter().any(|p| p.ink == Ink::Ash));
    }

    #[test]
    fn elapsed_reads_like_the_prototype() {
        assert_eq!(elapsed(0), "1s");
        assert_eq!(elapsed(7_999), "7s");
        assert_eq!(elapsed(372_000), "6m 12s");
        assert_eq!(elapsed(7_500_000), "2h 5m");
    }
}
