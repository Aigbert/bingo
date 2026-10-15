//! A durable journal prefix, including the selected turn's completion.
use bingo_sdk::*;

pub fn kept(frames: &[Frame], at: Option<&TurnId>) -> Result<Vec<Frame>, KernelError> {
    let end = match at {
        None => frames.len(),
        Some(at) => frames
            .iter()
            .position(|frame| {
                matches!(&frame.event,
            Event::TurnCompleted { turn, .. } if turn == at)
            })
            .map(|index| index + 1)
            .ok_or_else(|| {
                KernelError::new(ErrorCode::InvalidInput, "no completed turn at this cut")
            })?,
    };
    Ok(frames[..end].to_vec())
}

pub(crate) fn rewrite(frames: &mut [Frame], id: &SessionId, link: &ForkLink) {
    for frame in frames {
        frame.session = id.clone();
        if let Event::SessionUpdated { summary } = &mut frame.event {
            root(summary, id, link);
        }
    }
}

pub(crate) fn root(summary: &mut SessionSummary, id: &SessionId, link: &ForkLink) {
    summary.id = id.clone();
    summary.parent = None;
    summary.key = None;
    summary.history.forked_from = Some(link.clone());
    summary.busy = false;
    summary.asking = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_middle_unknown_and_rewind_records() {
        let frame = |seq, event| Frame {
            seq: Seq(seq),
            session: SessionId::from_raw("source"),
            ts: jiff::Timestamp::UNIX_EPOCH,
            cause: None,
            event,
        };
        let done = |turn| Event::TurnCompleted {
            turn: TurnId::from_raw(turn),
            status: TurnStatus::Completed,
            usage: Usage::default(),
        };
        let frames = vec![
            frame(
                1,
                Event::Rewound {
                    generation: 1,
                    to_turn: TurnId::from_raw("old"),
                    dropped: vec![ItemId::from_raw("old-item")],
                    files_restored: vec![],
                },
            ),
            frame(
                2,
                Event::ItemCompleted {
                    item: Item {
                        id: ItemId::from_raw("rewind-item"),
                        turn: Some(TurnId::from_raw("one")),
                        round: 0,
                        status: ItemStatus::Completed,
                        started_at: jiff::Timestamp::UNIX_EPOCH,
                        completed_at: Some(jiff::Timestamp::UNIX_EPOCH),
                        intent: None,
                        meta: Default::default(),
                        body: ItemBody::Rewind {
                            to_turn: TurnId::from_raw("old"),
                            dropped: 1,
                        },
                    },
                },
            ),
            frame(3, done("one")),
            frame(4, done("two")),
        ];
        assert_eq!(kept(&frames, None).unwrap(), frames);
        let mut mid = kept(&frames, Some(&TurnId::from_raw("one"))).unwrap();
        assert_eq!(mid, frames[..3]);
        assert_eq!(
            kept(&frames, Some(&TurnId::from_raw("missing")))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        rewrite(
            &mut mid,
            &SessionId::from_raw("fork"),
            &ForkLink {
                session: SessionId::from_raw("source"),
                turn: None,
            },
        );
        assert!(mid.iter().all(|f| f.session.as_str() == "fork"));
        assert_eq!(mid[0].event, frames[0].event);
    }
}
