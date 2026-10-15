//! Input typed ahead of the place that will read it (M180).
//!
//! A key waits here while the site it was typed at is still being drawn or
//! still judging the key before it, and while a command line that may open a
//! site is on its way to the kernel. Waiting keys leave in the order they were
//! typed. A key whose site has gone is addressed again to whatever owns the
//! keyboard now, so a burst that closes one site and opens the next reaches
//! the next — except a dialog: input typed elsewhere never enters it. Queued
//! pastes never enter a dialog. A notice counts the input events kept out.
use super::*;
use crossterm::event::KeyEvent;

/// How long keys wait behind a command line that has not been acknowledged.
/// A command that opens a site is acknowledged once it has run; one that runs
/// for longer than this hands the keyboard back rather than hold it.
pub(super) const OPENING_WAIT: Duration = Duration::from_secs(1);

/// Count input events without exposing their contents.
pub(super) fn kept_out_notice(count: usize) -> String {
    let inputs = if count == 1 { "input" } else { "inputs" };
    format!("Kept {count} queued {inputs} out of this dialog")
}

#[derive(Default)]
pub(super) struct Typeahead {
    pub(super) catalogue_pending: bool,
    inputs: std::collections::VecDeque<Queued>,
    opening: Option<Opening>,
}

/// One input event and the site it was typed at; `None` is the composer, or whatever
/// owns the keyboard with no site open.
struct Queued {
    site: Option<String>,
    input: Held,
}

enum Held {
    Key(KeyEvent),
    Paste(String),
}

/// A command line on its way: keys typed after it wait for its answer, which
/// may open the site they were meant for.
struct Opening {
    intent: IntentId,
    until: Instant,
}

impl Typeahead {
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.inputs.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Escape and ctrl+c end what was typed ahead along with everything else.
    pub(super) fn clear(&mut self) {
        self.inputs.clear();
        self.opening = None;
    }

    /// Keys typed at a dialog that has been answered go with it.
    pub(super) fn forget_site(&mut self, site: &str) {
        self.inputs
            .retain(|queued| queued.site.as_deref() != Some(site));
    }

    /// When the loop must wake to let keys go that waited out a command line.
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.opening.as_ref().map(|opening| opening.until)
    }

    #[cfg(test)]
    pub(super) fn expire(&mut self) {
        if let Some(opening) = &mut self.opening {
            opening.until = Instant::now();
        }
    }

    fn opening(&self, now: Instant) -> bool {
        self.opening
            .as_ref()
            .is_some_and(|opening| now < opening.until)
    }
}

/// A command line went out: what is typed next waits for its answer.
pub(super) fn command_sent(run: &mut Run, intent: IntentId) {
    run.typeahead.opening = Some(Opening {
        intent,
        until: run.detection.now().instant + OPENING_WAIT,
    });
}

/// The command line's answer is in, and whatever it opened is open.
pub(super) fn settled(run: &mut Run, intent: &IntentId) {
    if run
        .typeahead
        .opening
        .as_ref()
        .is_some_and(|opening| &opening.intent == intent)
    {
        run.typeahead.opening = None;
        drain(run);
    }
}

fn focus(run: &Run) -> Option<String> {
    site_keys::target(run)
        .filter(|site| {
            // An open ask may be set aside or covered by another keyboard owner.
            run.session.tree.open_interaction().is_none_or(|(_, open)| {
                site.focus != open.id.as_str()
                    || crate::focus::owner(&run.ui, &run.session.tree)
                        == crate::focus::Owner::Interaction
            })
        })
        .map(|site| site.focus)
}

fn drawing(run: &Run) -> bool {
    (crate::focus::owner(&run.ui, &run.session.tree) == crate::focus::Owner::Pane
        && crate::pane::readiness::waiting(&run.ui, run.session.tree.viewed()))
        || crate::browse::waiting(&run.ui, &run.session.tree)
        || crate::plugins::waiting(&run.ui)
        || crate::palette::waiting(&run.ui, &run.session.tree, run.detection.now())
        || crate::sessions::waiting(&run.ui, &run.session.tree, run.detection.now())
}

/// Whether a key typed now must wait behind something: a site still drawn,
/// a key still judged, a command line still out, or keys typed before it.
pub(super) fn blocked(run: &Run) -> bool {
    if drawing(run) || run.typeahead.opening(run.detection.now().instant) {
        return true;
    }
    !run.typeahead.is_empty() || focus(run).is_some_and(|focus| judging(run, &focus))
}

fn judging(run: &Run, site: &str) -> bool {
    run.site_keys.values().any(|pending| pending.focus == site)
}

pub(super) fn queue(run: &mut Run, key: KeyEvent) {
    // A closing palette can restore an ask before the command is acknowledged.
    // Keys still waiting for that command were not addressed to the ask.
    let site = focus(run).filter(|site| {
        !run.typeahead.opening(run.detection.now().instant) || !dialog(run, Some(site))
    });
    run.typeahead.inputs.push_back(Queued {
        site,
        input: Held::Key(key),
    });
}

/// A site closed while its handle was opening; this key precedes the held tail.
pub(super) fn retry(run: &mut Run, site: String, key: KeyEvent) {
    run.typeahead.inputs.push_front(Queued {
        site: Some(site),
        input: Held::Key(key),
    });
    drain(run);
}

/// A paste waits in the same order as keys, but never enters an ask from here.
pub(super) fn paste(run: &mut Run, text: String) {
    if blocked(run) {
        let site = focus(run);
        run.typeahead.inputs.push_back(Queued {
            site,
            input: Held::Paste(text),
        });
    } else {
        input::on_paste(&mut run.ui, &text);
    }
}

fn apply(run: &mut Run, held: Held) {
    run.behind = true;
    match held {
        Held::Key(key) => {
            let effects = input::on_key(&mut run.ui, &run.session.tree, key, run.detection.now());
            run.apply(effects);
        }
        Held::Paste(text) => input::on_paste(&mut run.ui, &text),
    }
}

/// Let waiting keys go, in order, for as long as nothing holds the first.
pub(super) fn drain(run: &mut Run) {
    // A command line that outran its wait no longer holds the keyboard, nor
    // a wake-up for the loop.
    if !run.typeahead.opening(run.detection.now().instant) {
        run.typeahead.opening = None;
    }
    let mut kept_out = 0;
    while let Some(front) = run.typeahead.inputs.front() {
        if matches!(&front.input, Held::Key(key) if catalogues::waiting(run, *key)) {
            break;
        }
        let site = front.site.clone();
        let now = focus(run);
        if matches!(front.input, Held::Paste(_)) && dialog(run, now.as_deref()) {
            run.typeahead.inputs.pop_front();
            kept_out += 1;
            continue;
        }
        if site != now {
            if dialog(run, now.as_deref()) {
                run.typeahead.inputs.pop_front();
                kept_out += 1;
            } else if let Some(front) = run.typeahead.inputs.front_mut() {
                front.site = now;
            }
            continue;
        }
        if waiting(run, site.as_deref()) {
            break;
        }
        if let Some(queued) = run.typeahead.inputs.pop_front() {
            apply(run, queued.input);
        }
    }
    if kept_out > 0 {
        run.behind = true;
        run.ui
            .notify(Level::Warn, kept_out_notice(kept_out), Instant::now());
    }
}

/// Whether the keyboard's owner is a dialog a key could answer, which a key
/// typed somewhere else is never re-addressed to. An ask set aside is not:
/// what is typed then goes to the composer.
fn dialog(run: &Run, focus: Option<&str>) -> bool {
    crate::focus::owner(&run.ui, &run.session.tree) == crate::focus::Owner::Interaction
        && focus.is_some_and(|focus| {
            run.session
                .tree
                .open_interaction()
                .is_some_and(|(_, open)| open.id.as_str() == focus)
        })
}

fn waiting(run: &Run, site: Option<&str>) -> bool {
    if drawing(run) || run.typeahead.opening(run.detection.now().instant) {
        return true;
    }
    if site.is_some_and(|site| judging(run, site)) {
        return true;
    }
    crate::focus::owner(&run.ui, &run.session.tree) == crate::focus::Owner::Interaction
        && run
            .session
            .tree
            .open_interaction()
            .is_some_and(|(owner, open)| crate::ask::waiting(&run.ui, owner, open))
}
