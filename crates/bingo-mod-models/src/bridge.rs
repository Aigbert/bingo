//! Command chaining uses acknowledgements; credentials only cross the private answer door.
use bingo_sdk::*;
use futures::StreamExt;
pub async fn attach(host: &HostHandle, session: &SessionId) -> Result<Attachment, KernelError> {
    host.open(
        SessionSelector::ById {
            id: session.clone(),
        },
        ClientIdentity {
            name: "bingo.mod.models".into(),
            surface: String::new(),
        },
        OpenOptions::default(),
    )
    .await
}
pub async fn run(
    host: &HostHandle,
    session: &SessionId,
    name: &str,
    args: &str,
    key: Option<(String, String)>,
    surface: Option<render::SurfaceName>,
) -> Result<(), KernelError> {
    let mut attached = attach(host, session).await?;
    let intent = IntentId::mint();
    let mut origin = Origin::surface("plugin:bingo.mod.models");
    origin.render_surface = surface;
    attached.handle.submit(
        intent.clone(),
        Input::Action {
            origin: Some(origin),
            action: Action {
                name: name.into(),
                args: args.into(),
            },
        },
    );
    let mut key = key;
    while let Some(frame) = attached.events.next().await {
        match frame.event {
            Event::IntentAck {
                intent: ack,
                outcome,
            } if ack == intent => {
                return match outcome {
                    IntentOutcome::Rejected { error } => Err(error),
                    IntentOutcome::Applied { .. } => Ok(()),
                    _ => continue,
                };
            }
            Event::InteractionOpened { interaction } => {
                if let InteractionKind::Login {
                    provider,
                    flow: LoginFlow::Paste,
                } = &interaction.kind
                    && key.as_ref().is_some_and(|(id, _)| id == provider)
                    && let Some((_, secret)) = key.take()
                {
                    attached.handle.answer(
                        IntentId::mint(),
                        interaction.id,
                        Answer::Text { text: secret },
                        Activation::Programmatic,
                    );
                }
            }
            Event::SessionClosed { .. } => return Err(crate::error("session closed")),
            _ => {}
        }
    }
    Err(crate::error("session closed before the command answered"))
}
pub async fn paste(
    host: &HostHandle,
    session: &SessionId,
    title: &str,
) -> Result<Option<String>, KernelError> {
    let answer = host
        .ask(
            session,
            InteractionKind::Login {
                provider: title.into(),
                flow: LoginFlow::Paste,
            },
            vec![AnswerSpec::Text, AnswerSpec::Cancel],
        )
        .await?;
    Ok(match answer {
        Answer::Text { text } => Some(text),
        _ => None,
    })
}
