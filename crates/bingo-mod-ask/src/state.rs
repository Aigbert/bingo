//! Where the person is in a question: the tab on screen, the choice each
//! tab is on, what is ticked, what is answered, and the words being typed.
use bingo_sdk::{Answer, Question};

#[derive(Clone, Debug, Default)]
pub struct State {
    pub tab: usize,
    pub fields: Vec<Field>,
    /// Words typed into the current tab's `Something else`.
    pub words: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct Field {
    pub cursor: usize,
    /// Ticked options of a multiple choice, by index.
    pub ticks: Vec<usize>,
    pub answer: Option<Answer>,
}

/// What a key did: whether it was used, and every answer once the last
/// question is answered.
pub type Outcome = (bool, Option<Vec<Answer>>);

/// The rows a question offers: its options, then its own words.
pub fn choices(question: &Question) -> usize {
    question.options.len() + usize::from(question.free_text)
}

impl State {
    pub fn field(&self, tab: usize) -> Field {
        self.fields.get(tab).cloned().unwrap_or_default()
    }

    pub fn key(&mut self, questions: &[Question], key: &str) -> Outcome {
        self.fields.resize_with(questions.len(), Field::default);
        self.tab = self.tab.min(questions.len().saturating_sub(1));
        let Some(question) = questions.get(self.tab) else {
            return (false, None);
        };
        if self.words.is_some() {
            return self.typing(key);
        }
        let field = &mut self.fields[self.tab];
        let last = choices(question).saturating_sub(1);
        match key {
            "left" => self.tab = self.tab.saturating_sub(1),
            "right" => self.tab = (self.tab + 1).min(questions.len() - 1),
            "up" => field.cursor = field.cursor.saturating_sub(1),
            "down" => field.cursor = (field.cursor + 1).min(last),
            " " | "space" if question.multi => tick(field, question),
            "enter" => return self.choose(questions),
            _ => return self.start_words(question, key),
        }
        (true, None)
    }

    /// A click on a choice: the cursor goes to it and it is chosen, as
    /// `enter` on it would. Words being typed keep the keys to themselves.
    pub fn pick(&mut self, questions: &[Question], id: &str) -> Outcome {
        self.fields.resize_with(questions.len(), Field::default);
        self.tab = self.tab.min(questions.len().saturating_sub(1));
        let Some(question) = questions.get(self.tab) else {
            return (false, None);
        };
        let Some(index) = id.parse().ok().filter(|index| *index < choices(question)) else {
            return (false, None);
        };
        if self.words.is_some() {
            return (false, None);
        }
        self.fields[self.tab].cursor = index;
        self.choose(questions)
    }

    /// A printable key on the list starts `Something else` with it.
    fn start_words(&mut self, question: &Question, key: &str) -> Outcome {
        let Some(typed) = printable(key).filter(|_| question.free_text) else {
            return (false, None);
        };
        self.fields[self.tab].cursor = question.options.len();
        self.words = Some(typed.to_owned());
        (true, None)
    }

    fn choose(&mut self, questions: &[Question]) -> Outcome {
        let question = &questions[self.tab];
        let field = &mut self.fields[self.tab];
        if field.cursor >= question.options.len() {
            self.words = question.free_text.then(String::new);
            return (true, None);
        }
        if question.multi && field.ticks.is_empty() {
            field.ticks.push(field.cursor);
        }
        field.answer = Some(chosen(field, question));
        (true, self.next())
    }

    fn typing(&mut self, key: &str) -> Outcome {
        let Some(words) = self.words.as_mut() else {
            return (false, None);
        };
        match key {
            "esc" => self.words = None,
            "enter" => {
                let text = words.trim().to_owned();
                self.words = None;
                if text.is_empty() {
                    return (true, None);
                }
                self.fields[self.tab].answer = Some(Answer::Text { text });
                return (true, self.next());
            }
            "backspace" => {
                words.pop();
            }
            _ => match printable(key) {
                Some(typed) => words.push_str(typed),
                None => return (false, None),
            },
        }
        (true, None)
    }

    /// The first question left unanswered comes on screen; with none left,
    /// every answer goes back together.
    fn next(&mut self) -> Option<Vec<Answer>> {
        match self.fields.iter().position(|field| field.answer.is_none()) {
            Some(tab) => {
                self.tab = tab;
                None
            }
            None => self
                .fields
                .iter()
                .map(|field| field.answer.clone())
                .collect(),
        }
    }
}

/// One character the person typed, shift folded in.
fn printable(key: &str) -> Option<&str> {
    let key = key.strip_prefix("shift+").unwrap_or(key);
    match key {
        "space" => Some(" "),
        _ if key.chars().count() == 1 => Some(key),
        _ => None,
    }
}

fn tick(field: &mut Field, question: &Question) {
    if field.cursor >= question.options.len() {
        return;
    }
    match field.ticks.iter().position(|at| *at == field.cursor) {
        Some(at) => {
            field.ticks.remove(at);
        }
        None => field.ticks.push(field.cursor),
    }
}

fn chosen(field: &Field, question: &Question) -> Answer {
    let mut ticked = field.ticks.clone();
    ticked.sort_unstable();
    let picked = if question.multi {
        ticked
    } else {
        vec![field.cursor]
    };
    Answer::Choice {
        ids: picked
            .into_iter()
            .filter_map(|at| question.options.get(at).map(|option| option.id.clone()))
            .collect(),
        other: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn question(multi: bool) -> Question {
        serde_json::from_value(json!({"question":"Which?","options":[{"id":"a","label":"Alpha"},{"id":"b","label":"Beta"}],"multi":multi,"freeText":true})).unwrap()
    }
    fn choice(ids: &[&str]) -> Answer {
        Answer::Choice {
            ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            other: None,
        }
    }

    #[test]
    fn enter_answers_and_moves_to_the_next_question_then_finishes() {
        let questions = [question(false), question(true)];
        let mut state = State::default();
        state.key(&questions, "down");
        assert_eq!(state.key(&questions, "enter"), (true, None));
        assert_eq!(state.tab, 1);
        state.key(&questions, " ");
        state.key(&questions, "down");
        state.key(&questions, " ");
        assert_eq!(
            state.key(&questions, "enter").1,
            Some(vec![choice(&["b"]), choice(&["a", "b"])])
        );
    }

    #[test]
    fn a_pick_answers_as_enter_on_that_choice_does() {
        let questions = [question(false)];
        let mut state = State::default();
        assert_eq!(state.pick(&questions, "9"), (false, None));
        assert_eq!(state.pick(&questions, "1").1, Some(vec![choice(&["b"])]));
        let mut state = State::default();
        assert_eq!(state.pick(&questions, "2"), (true, None));
        assert_eq!(state.words.as_deref(), Some(""), "the words row opens");
    }

    #[test]
    fn digits_are_words_not_answers() {
        let questions = [question(false)];
        let mut state = State::default();
        assert_eq!(state.key(&questions, "2"), (true, None));
        assert_eq!(state.words.as_deref(), Some("2"));
        assert_eq!(state.field(0).cursor, 2);
        state.key(&questions, "x");
        assert_eq!(
            state.key(&questions, "enter").1,
            Some(vec![Answer::Text { text: "2x".into() }])
        );
    }

    #[test]
    fn esc_leaves_the_words_and_an_empty_enter_stays() {
        let questions = [question(false)];
        let mut state = State::default();
        state.key(&questions, "down");
        state.key(&questions, "down");
        assert_eq!(state.key(&questions, "enter"), (true, None));
        assert_eq!(state.words.as_deref(), Some(""));
        assert_eq!(state.key(&questions, "enter"), (true, None));
        assert!(state.words.is_none());
        state.key(&questions, "q");
        state.key(&questions, "esc");
        assert!(state.words.is_none());
    }

    #[test]
    fn arrows_walk_tabs_and_a_multiple_choice_ticks_the_cursor_when_none_is() {
        let questions = [question(true), question(false)];
        let mut state = State::default();
        state.key(&questions, "right");
        state.key(&questions, "right");
        assert_eq!(state.tab, 1);
        state.key(&questions, "left");
        state.key(&questions, "down");
        state.key(&questions, "enter");
        assert_eq!(state.field(0).answer, Some(choice(&["b"])));
        assert_eq!(state.tab, 1);
    }
}
