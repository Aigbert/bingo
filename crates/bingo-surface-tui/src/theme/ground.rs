//! Terminal palette replies.
pub(crate) fn answered_ground(reply: &[u8]) -> Option<(u16, u16, u16)> {
    answered(reply, b"\x1b]11;")
}

/// The colour spec inside the reply, between `ESC ] 11 ;` and whichever
/// terminator the terminal chose.
fn spec<'a>(reply: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    let body = reply.strip_prefix(prefix)?;
    let body = body
        .strip_suffix(b"\x1b\\")
        .or_else(|| body.strip_suffix(b"\x07"))?;
    (!body.is_empty()).then_some(body)
}

/// An X11 colour string as `xparsecolor` reads the ones terminals answer with,
/// which is the shape `terminal_colorsaurus` accepted for the first answer:
/// `rgb:` and one to four hex digits a channel, or the older `#` form.
fn colour(spec: &[u8]) -> Option<(u16, u16, u16)> {
    let text = std::str::from_utf8(spec).ok()?;
    match text.strip_prefix("rgb:") {
        Some(written) => channels(written),
        None => shifted(text.strip_prefix('#')?),
    }
}

/// `rgb:<red>/<green>/<blue>`, each channel scaled from the width it was
/// written in: `h` is four bits, `hhhh` sixteen. A fourth channel is `rgba:`,
/// which this question is never answered with.
fn channels(written: &str) -> Option<(u16, u16, u16)> {
    let mut parts = written.split('/');
    let (red, green, blue) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    Some((scale(red)?, scale(green)?, scale(blue)?))
}

fn scale(digits: &str) -> Option<u16> {
    let width = u32::try_from(digits.len()).ok()?;
    if !(1..=4).contains(&width) {
        return None;
    }
    let most = 16u32.pow(width) - 1;
    let value = u32::from_str_radix(digits, 16).ok()?;
    u16::try_from(u32::from(u16::MAX) * value / most).ok()
}

/// The older `#<red><green><blue>`, whose digits are the most significant bits
/// of each channel rather than a scale: `#3a7` is `#3000a0007000`.
fn shifted(digits: &str) -> Option<(u16, u16, u16)> {
    let width = digits.len() / 3;
    if !digits.len().is_multiple_of(3) || !(1..=4).contains(&width) {
        return None;
    }
    let channel = |n: usize| {
        let part = digits.get(n * width..(n + 1) * width)?;
        Some(u16::from_str_radix(part, 16).ok()? << ((4 - width) * 4))
    };
    Some((channel(0)?, channel(1)?, channel(2)?))
}

/// Whether a ground of this colour is a light one, by the lightness
/// `terminal_colorsaurus` measures — the same maths the first answer went
/// through, so the probe and a later reply cannot disagree about one colour.
///
/// The probe has the ink beside the ground and compares the two; a reply to
/// this one question carries only the ground, and the threshold is then the
/// perceptual middle grey the crate falls back to when ink and ground are
/// worth the same.
pub fn background_is_light(red: u16, green: u16, blue: u16) -> bool {
    terminal_colorsaurus::Color::rgb(red, green, blue).perceived_lightness() > 0.5
}

pub(crate) fn answered_foreground(reply: &[u8]) -> Option<(u16, u16, u16)> {
    answered(reply, b"\x1b]10;")
}
fn answered(reply: &[u8], prefix: &[u8]) -> Option<(u16, u16, u16)> {
    colour(spec(reply, prefix)?)
}
