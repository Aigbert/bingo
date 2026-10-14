use bingo_sdk::Interaction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jiff::Timestamp;

pub const RECENT_MS: i64 = 900;

pub fn recent(opened: Timestamp, typed: Option<Timestamp>) -> bool {
    typed.is_some_and(|typed| {
        let elapsed = opened.as_millisecond() - typed.as_millisecond();
        (0..RECENT_MS).contains(&elapsed)
    })
}

pub fn printable(key: KeyEvent, interaction: &Interaction) -> bool {
    if !(key.modifiers - KeyModifiers::SHIFT).is_empty() {
        return false;
    }
    let KeyCode::Char(c) = key.code else {
        return false;
    };
    let options = super::options::options(interaction);
    if c.to_digit(10)
        .is_some_and(|n| n > 0 && n as usize <= options.len())
    {
        return false;
    }
    if matches!(
        interaction.kind,
        bingo_sdk::InteractionKind::Permission { .. }
    ) && matches!(c, 'y' | 'n' | 'a')
    {
        return false;
    }
    if c == ' '
        && matches!(
            interaction.kind,
            bingo_sdk::InteractionKind::Question(bingo_sdk::Question { multi: true, .. })
        )
    {
        return false;
    }
    !matches!(
        interaction.kind,
        bingo_sdk::InteractionKind::Form { .. } | bingo_sdk::InteractionKind::Login { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_typing_uses_an_exact_window_without_wall_time() {
        let opened = Timestamp::from_millisecond(10_000).unwrap();
        for (delta, expected) in [
            (0, true),
            (899, true),
            (900, false),
            (901, false),
            (-1, false),
        ] {
            assert_eq!(
                recent(
                    opened,
                    Some(Timestamp::from_millisecond(10_000 - delta).unwrap())
                ),
                expected
            );
        }
        assert!(!recent(opened, None));
    }
    #[test]
    fn unrecognized_printing_sets_aside_without_answering() {
        let open = crate::test_support::permission(None, None);
        for (key, expected) in [
            ('1', false),
            ('2', false),
            ('x', true),
            ('9', true),
            ('y', false),
        ] {
            assert_eq!(
                printable(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), &open),
                expected
            );
        }
    }
}
