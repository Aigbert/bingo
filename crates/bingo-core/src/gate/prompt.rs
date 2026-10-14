use super::*;

pub(super) enum Reply {
    Changed(serde_json::Value),
    Decided(Gate, DecisionKind, Option<String>),
}

pub(super) async fn ask_person(
    input: PolicyInput<'_>,
    tool: &dyn Tool,
    scope: Option<String>,
    blocked: Option<String>,
    policy: &dyn PermissionPolicy,
    prompter: &dyn Prompter,
) -> Reply {
    let caution = policy.caution(input).or_else(|| {
        input
            .traits
            .destructive
            .then(|| "this call can irreversibly destroy data".into())
    });
    let scope = scope.filter(|_| caution.is_none() && blocked.is_none());
    let answers = answers(
        policy.allows_replacement(input),
        scope.is_some(),
        blocked.is_some(),
    );
    let kind = InteractionKind::Permission {
        tool: input.call.name.clone(),
        summary: blocked
            .clone()
            .or_else(|| input.confirm.map(str::to_owned))
            .unwrap_or_else(|| summarize(input.call, input.subjects)),
        preview: tool.preview(&input.call.input, input.cwd),
        session_scope: scope.clone(),
        caution: blocked.or(caution),
    };
    let answer = prompter
        .ask(kind, answers.clone())
        .await
        .unwrap_or(Answer::Cancel);
    if !answers.contains(&answer.spec()) {
        return denied(None);
    }
    match answer {
        Answer::AllowChanged { input } => Reply::Changed(input),
        Answer::AllowOnce => Reply::Decided(Gate::Allowed, DecisionKind::Allow, None),
        Answer::AllowSession { scope: requested } if scope.as_ref() == Some(&requested) => {
            policy.on_verdict(input, &Verdict::Allow { scope }).await;
            Reply::Decided(Gate::Allowed, DecisionKind::AllowSession, None)
        }
        Answer::Deny { feedback } => {
            policy
                .on_verdict(
                    input,
                    &Verdict::Deny {
                        feedback: feedback.clone(),
                    },
                )
                .await;
            denied(feedback)
        }
        _ => denied(None),
    }
}

fn answers(editable: bool, scoped: bool, blocked: bool) -> Vec<AnswerSpec> {
    let mut answers = Vec::new();
    if !blocked {
        answers.push(AnswerSpec::AllowOnce);
    }
    if scoped {
        answers.push(AnswerSpec::AllowSession);
    }
    answers.push(AnswerSpec::Deny);
    if editable {
        answers.push(AnswerSpec::AllowChanged);
    }
    answers
}

fn denied(feedback: Option<String>) -> Reply {
    let mut message = "Permission denied by the user".to_string();
    if let Some(feedback) = &feedback {
        message.push_str(&format!(": {feedback}"));
    }
    Reply::Decided(Gate::Denied { message }, DecisionKind::Deny, feedback)
}
