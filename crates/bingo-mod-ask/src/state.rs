use bingo_sdk::{Answer, Question};

pub fn fields(count: usize, titled: bool) -> bool {
    titled || count > 1
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub cursor: usize,
    pub fields: Vec<Field>,
}
#[derive(Clone, Debug, Default)]
pub struct Field {
    pub selected: usize,
    pub ticks: Vec<String>,
    pub words: Option<String>,
}

impl State {
    pub fn key(&mut self, questions: &[Question], form: bool, key: &str) -> (bool, Option<Answer>) {
        self.fields.resize_with(questions.len(), Field::default);
        if questions.is_empty() {
            return (false, None);
        }
        if form {
            return self.form_key(questions, key);
        }
        self.choice_key(&questions[0], key)
    }
    fn choice_key(&mut self, question: &Question, key: &str) -> (bool, Option<Answer>) {
        let field = &mut self.fields[0];
        if let Some(words) = &mut field.words {
            if key == "enter" {
                return (true, Some(answer(field, question)));
            }
            return (edit(words, key), None);
        }
        let count = question.options.len() + usize::from(question.free_text);
        match key {
            "up" => field.selected = field.selected.saturating_sub(1),
            "down" => field.selected = (field.selected + 1).min(count.saturating_sub(1)),
            "space" | " " if question.multi => tick(field, question),
            "enter" => return (true, choose(field, question)),
            _ => {
                let Some(index) = number(key, count) else {
                    return (false, None);
                };
                field.selected = index;
                if question.multi && index < question.options.len() {
                    tick(field, question);
                } else {
                    return (true, choose(field, question));
                }
            }
        }
        (true, None)
    }
    fn form_key(&mut self, questions: &[Question], key: &str) -> (bool, Option<Answer>) {
        self.cursor = self.cursor.min(questions.len() - 1);
        match key {
            "tab" => self.cursor = (self.cursor + 1) % questions.len(),
            "shift+tab" => self.cursor = (self.cursor + questions.len() - 1) % questions.len(),
            "enter" => {
                return (
                    true,
                    Some(Answer::Form {
                        answers: questions
                            .iter()
                            .zip(&self.fields)
                            .map(|(q, f)| answer(f, q))
                            .collect(),
                    }),
                );
            }
            _ => {
                return (
                    field_key(&mut self.fields[self.cursor], &questions[self.cursor], key),
                    None,
                );
            }
        }
        (true, None)
    }
}
fn number(key: &str, count: usize) -> Option<usize> {
    let n = key.parse::<usize>().ok()?.checked_sub(1)?;
    (n < count).then_some(n)
}
fn field_key(field: &mut Field, question: &Question, key: &str) -> bool {
    match key {
        "left" => field.selected = field.selected.saturating_sub(1),
        "right" => {
            field.selected = (field.selected + 1).min(question.options.len().saturating_sub(1))
        }
        "space" | " " if question.multi => tick(field, question),
        "space" | " " if question.options.len() == 2 => field.selected = (field.selected + 1) % 2,
        _ if question.options.is_empty() || question.free_text => {
            return edit(field.words.get_or_insert_default(), key);
        }
        _ => return false,
    }
    true
}
fn edit(words: &mut String, key: &str) -> bool {
    let key = key
        .strip_prefix("shift+")
        .filter(|key| key.chars().count() == 1)
        .unwrap_or(key);
    if key == "backspace" {
        words.pop();
        return true;
    }
    let value = if key == "space" { " " } else { key };
    if value.chars().count() == 1 {
        words.push_str(value);
        return true;
    }
    false
}
fn tick(field: &mut Field, question: &Question) {
    let Some(option) = question.options.get(field.selected) else {
        return;
    };
    if let Some(at) = field.ticks.iter().position(|id| id == &option.id) {
        field.ticks.remove(at);
    } else {
        field.ticks.push(option.id.clone());
    }
}
fn choose(field: &mut Field, question: &Question) -> Option<Answer> {
    if question.free_text && field.selected == question.options.len() {
        field.words = Some(String::new());
        None
    } else {
        Some(answer(field, question))
    }
}
fn answer(field: &Field, question: &Question) -> Answer {
    let other = field
        .words
        .as_ref()
        .filter(|words| !words.trim().is_empty())
        .cloned();
    let ids = if question.multi {
        field.ticks.clone()
    } else if other.is_some() {
        vec![]
    } else {
        question
            .options
            .get(field.selected)
            .map(|option| vec![option.id.clone()])
            .unwrap_or_default()
    };
    if ids.is_empty() {
        other
            .map(|text| Answer::Text { text })
            .unwrap_or(Answer::Cancel)
    } else {
        Answer::Choice { ids, other }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn question(multi: bool) -> Question {
        serde_json::from_value(json!({"question":"Which?","options":[{"id":"a","label":"Alpha"},{"id":"b","label":"Beta"}],"multi":multi,"freeText":true})).unwrap()
    }
    #[test]
    fn numbers_answer_and_words_or_ticks_preserve_their_values() {
        let mut state = State::default();
        let questions = [question(false)];
        assert_eq!(
            state.key(&questions, false, "2").1,
            Some(Answer::Choice {
                ids: vec!["b".into()],
                other: None
            })
        );
        assert_eq!(state.key(&questions, false, "x"), (false, None));
        state.key(&questions, false, "3");
        state.key(&questions, false, "x");
        assert_eq!(
            state.key(&questions, false, "enter").1,
            Some(Answer::Text { text: "x".into() })
        );
        let mut state = State::default();
        let questions = [question(true)];
        state.key(&questions, false, " ");
        state.key(&questions, false, "down");
        state.key(&questions, false, " ");
        assert_eq!(
            state.key(&questions, false, "enter").1,
            Some(Answer::Choice {
                ids: vec!["a".into(), "b".into()],
                other: None
            })
        );
    }
    #[test]
    fn a_form_walks_fields_then_submits_all_of_them_once() {
        let questions = [question(false), question(true)];
        let mut state = State::default();
        state.key(&questions, true, "right");
        state.key(&questions, true, "tab");
        state.key(&questions, true, " ");
        assert_eq!(
            state.key(&questions, true, "enter").1,
            Some(Answer::Form {
                answers: vec![
                    Answer::Choice {
                        ids: vec!["b".into()],
                        other: None
                    },
                    Answer::Choice {
                        ids: vec!["a".into()],
                        other: None
                    }
                ]
            })
        );
    }
}
