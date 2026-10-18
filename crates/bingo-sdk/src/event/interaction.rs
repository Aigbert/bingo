//! An interaction: a question the session holds open for a person or a
//! client: its kinds, its answers and how it ends.

use jiff::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ids::{InteractionId, ItemId, SessionId, TurnId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub id: InteractionId,
    pub session: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<TurnId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemId>,
    #[schemars(with = "String")]
    pub opened_at: Timestamp,
    /// Keyboard approvals before this instant are rejected with `NOT_READY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub guard_until: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub expires_at: Option<Timestamp>,
    pub kind: InteractionKind,
    /// Exactly the answers the kernel will accept.
    pub answers: Vec<AnswerSpec>,
}

/// One way to answer an interaction: the answer picking it sends, and the
/// words that name it. What a surface draws for a rung — a button, a numbered
/// line, a keystroke — is its own business; whether the rung is there at all
/// is the kernel's, and [`Interaction::rungs`] is where that is read.
#[derive(Clone, Debug, PartialEq)]
pub struct Rung {
    pub answer: Answer,
    pub label: String,
    /// The longer words the asker wrote under an option, where it wrote any.
    pub description: Option<String>,
}

impl Interaction {
    /// The answers this interaction offers, in the order they are shown:
    /// exactly the ones `answers` allows, so a row can never promise
    /// something the kernel would refuse. Words of one's own are not among
    /// them — they are a door and not an answer waiting to be sent — so a
    /// surface reads [`AnswerSpec::Text`] for that itself.
    ///
    /// A form is answered question by question rather than by picking one of
    /// a list, so it has no rungs.
    pub fn rungs(&self) -> Vec<Rung> {
        match &self.kind {
            InteractionKind::Permission {
                session_scope,
                caution,
                ..
            } => self.permission_rungs(session_scope.as_deref().filter(|_| caution.is_none())),
            InteractionKind::Question(question) => self.question_rungs(question),
            InteractionKind::Confirm { .. } => self
                .rung(Answer::Confirm, "Yes")
                .into_iter()
                .chain(self.rung(Answer::Cancel, "No"))
                .collect(),
            InteractionKind::Login { .. } => {
                self.rung(Answer::Cancel, "Cancel").into_iter().collect()
            }
            InteractionKind::Form { .. } => Vec::new(),
        }
    }

    /// One rung, when the kernel would take the answer it sends.
    fn rung(&self, answer: Answer, label: &str) -> Option<Rung> {
        self.answers.contains(&answer.spec()).then(|| Rung {
            answer,
            label: label.to_string(),
            description: None,
        })
    }

    /// The permission rungs, widest first. `AllowSession` without a scope
    /// would install no rule, so it is not offered as if it would.
    fn permission_rungs(&self, session_scope: Option<&str>) -> Vec<Rung> {
        let session = session_scope.and_then(|scope| {
            self.rung(
                Answer::AllowSession {
                    scope: scope.to_string(),
                },
                &format!("Allow {scope} for this session"),
            )
        });
        self.rung(Answer::AllowOnce, "Allow once")
            .into_iter()
            .chain(session)
            .chain(self.rung(Answer::Deny { feedback: None }, "Deny"))
            .collect()
    }

    /// One rung per option the asker named, in its own words.
    fn question_rungs(&self, question: &Question) -> Vec<Rung> {
        question
            .options
            .iter()
            .filter_map(|option| {
                let answer = Answer::Choice {
                    ids: vec![option.id.clone()],
                    other: None,
                };
                Some(Rung {
                    description: option.description.clone(),
                    ..self.rung(answer, &option.label)?
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InteractionKind {
    Permission {
        tool: String,
        /// The call after hooks and interceptors, before this decision.
        #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
        input: serde_json::Value,
        summary: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preview: Option<Preview>,
        /// The rule `AllowSession` would install, when one would silence the prompt.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_scope: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "read_caution"
        )]
        caution: Option<Caution>,
    },
    /// One question on its own. It holds [`Question`] flattened, so the frame
    /// it makes is the one it always was.
    Question(Question),
    /// A set of questions put at once and answered together (M53). Every
    /// question is the same [`Question`] a lone one is.
    Form {
        /// What the whole set is about, in the asker's own words: an MCP
        /// server's elicitation says which server is asking. A set the model
        /// opened has nothing to add to its own questions and names none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        questions: Vec<Question>,
    },
    Confirm {
        title: String,
        detail: String,
    },
    Login {
        provider: String,
        flow: LoginFlow,
    },
}

impl InteractionKind {
    /// The answer that picks the option in this role, when the question names
    /// one (ADR-0039 §2): what a door may answer without asking anybody. A
    /// form is answered only when every one of its questions names the option.
    pub fn answer_for(&self, role: AnswerRole) -> Option<Answer> {
        match self {
            InteractionKind::Question(question) => question.answer_for(role),
            InteractionKind::Form { questions, .. } => Some(Answer::Form {
                answers: questions
                    .iter()
                    .map(|question| question.answer_for(role))
                    .collect::<Option<Vec<_>>>()?,
            }),
            _ => None,
        }
    }
}

/// One question put to a person: what is asked, what may be picked, and
/// whether words of one's own or more than one option count. A lone question
/// and one of a form are the same thing (M53).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub question: String,
    /// A short tag a surface may show before the question (`Auth method`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub free_text: bool,
    #[serde(default)]
    pub multi: bool,
}

impl Question {
    /// The choice that picks this question's option in the role, when it
    /// names one (ADR-0039 §2).
    pub fn answer_for(&self, role: AnswerRole) -> Option<Answer> {
        let option = self
            .options
            .iter()
            .find(|option| option.role == Some(role))?;
        Some(Answer::Choice {
            ids: vec![option.id.clone()],
            other: None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What picking it would mean to a session that answers for the person
    /// (ADR-0039 §2). Unmarked options are a person's alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<AnswerRole>,
    /// A few lines of monospace shown beside the option while the cursor is
    /// on it — the layout or the snippet picking it would mean (M53).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// The two options a session may answer a question with when nobody is asked
/// (ADR-0039 §2). The asker names them in its own words; the kernel reads
/// which is which and never the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AnswerRole {
    /// "Let it happen."
    Allowing,
    /// The refusal a question falls to when nobody answers it.
    Refusing,
}

/// Why a call needs care, and the act its one-time approval names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Caution {
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<String>,
}

fn read_caution<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Caution>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Read {
        Legacy(String),
        Named(Caution),
    }
    Ok(
        Option::<Read>::deserialize(deserializer)?.map(|value| match value {
            Read::Legacy(reason) => Caution { reason, act: None },
            Read::Named(caution) => caution,
        }),
    )
}

/// The local consequences a command preview can establish before it runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Stakes {
    pub line: String,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Preview {
    Diff {
        unified: String,
    },
    Command {
        command: String,
        cwd: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stakes: Option<Stakes>,
    },
    Url {
        url: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LoginFlow {
    Browser { url: String },
    Device { url: String, code: String },
    Paste,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AnswerSpec {
    AllowChanged,
    AllowOnce,
    AllowSession,
    Deny,
    Choice,
    Text,
    Confirm,
    Cancel,
    Form,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Answer {
    AllowChanged {
        input: serde_json::Value,
    },
    AllowOnce,
    AllowSession {
        scope: String,
    },
    Deny {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        feedback: Option<String>,
    },
    /// The options ticked and, beside them, the words a person typed of their
    /// own (M59): a question may be answered both ways at once, and dropping
    /// either half loses what they said. An old frame carries no `other` and
    /// reads as the ticks alone. Words with nothing ticked are
    /// [`Answer::Text`], which is what a question answered in words alone has
    /// always been.
    Choice {
        ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        other: Option<String>,
    },
    Text {
        text: String,
    },
    Confirm,
    Cancel,
    /// A form's answers, one per question in the order they were asked; a
    /// question nobody answered is [`Answer::Cancel`] in its place (M53).
    Form {
        answers: Vec<Answer>,
    },
}

impl Answer {
    pub fn spec(&self) -> AnswerSpec {
        match self {
            Answer::AllowChanged { .. } => AnswerSpec::AllowChanged,
            Answer::AllowOnce => AnswerSpec::AllowOnce,
            Answer::AllowSession { .. } => AnswerSpec::AllowSession,
            Answer::Deny { .. } => AnswerSpec::Deny,
            Answer::Choice { .. } => AnswerSpec::Choice,
            Answer::Text { .. } => AnswerSpec::Text,
            Answer::Confirm => AnswerSpec::Confirm,
            Answer::Cancel => AnswerSpec::Cancel,
            Answer::Form { .. } => AnswerSpec::Form,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Activation {
    Keyboard,
    Pointer,
    Programmatic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ResolvedBy {
    Client { name: String, surface: String },
    Plugin { name: String, surface: String },
    Kernel,
    Policy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CancelReason {
    TurnEnded,
    /// The holding command that asked has finished (ADR-0012 §5).
    CommandEnded,
    Interrupted,
    SessionClosed,
    Expired,
    Superseded,
}

#[cfg(test)]
mod changed_contract {
    use super::*;
    use serde_json::json;

    #[test]
    fn changed_input_is_an_additive_answer() {
        let wire = json!({"kind":"allowChanged","input":{"command":"cargo test"}});
        let answer: Answer = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(answer.spec(), AnswerSpec::AllowChanged);
        assert_eq!(serde_json::to_value(answer).unwrap(), wire);
    }

    #[test]
    fn old_permissions_have_no_caution() {
        let wire = json!({"kind":"permission","tool":"Bash","summary":"run"});
        let kind: InteractionKind = serde_json::from_value(wire.clone()).unwrap();
        assert!(matches!(
            kind,
            InteractionKind::Permission { caution: None, .. }
        ));
        assert_eq!(serde_json::to_value(kind).unwrap(), wire);
    }
}
