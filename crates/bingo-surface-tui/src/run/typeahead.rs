//! Keys typed ahead of the place that will read them (M180).
//!
//! A key waits here while the site it was typed at is still being drawn or
//! still judging the key before it, and while a command line that may open a
//! site is on its way to the kernel. Waiting keys leave in the order they were
//! typed. A key whose site has gone is addressed again to whatever owns the
//! keyboard now, so a burst that closes one site and opens the next reaches
//! the next — except a dialog: no key typed before a dialog opened is typed
//! into it, and those keys are dropped with a notice that says how many.
use super::*;
use crossterm::event::KeyEvent;

/// How long keys wait behind a command line that has not been acknowledged.
/// A command that opens a site is acknowledged once it has run; one that runs
/// for longer than this hands the keyboard back rather than hold it.
pub(super) const OPENING_WAIT: Duration = Duration::from_secs(1);

/// What a dropped typeahead key says, with the count of keys it stands for.
pub(super) fn kept_out_notice(count: usize) -> String {
    let keys = if count == 1 { "key" } else { "keys" };
    format!("{count} {keys} typed before this dialog opened were not entered into it")
}

#[derive(Default)]
pub(super) struct Typeahead {
    keys: std::collections::VecDeque<Queued>,
    opening: Option<Opening>,
}

/// One key and the site it was typed at; `None` is the composer, or whatever
/// owns the keyboard with no site open.
struct Queued {
    site: Option<String>,
    key: KeyEvent,
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
        self.keys.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Escape and ctrl+c end what was typed ahead along with everything else.
    pub(super) fn clear(&mut self) {
        self.keys.clear();
        self.opening = None;
    }

    /// Keys typed at a dialog that has been answered go with it.
    pub(super) fn forget_site(&mut self, site: &str) {
        self.keys
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
    // An unhandled printable key sets the ask aside; finish its already queued
    // burst in the composer instead of dropping the remaining letters.
    site_keys::target(run)
        .map(|(_, focus, _)| focus)
        .or_else(|| {
            run.session
                .tree
                .open_interaction()
                .map(|(_, open)| open.id.to_string())
        })
}

fn drawing(run: &Run) -> bool {
    crate::browse::waiting(&run.ui, &run.session.tree)
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
    let site = focus(run);
    run.typeahead.keys.push_back(Queued { site, key });
}

/// Let waiting keys go, in order, for as long as nothing holds the first.
pub(super) fn drain(run: &mut Run) {
    // A command line that outran its wait no longer holds the keyboard, nor
    // a wake-up for the loop.
    if !run.typeahead.opening(run.detection.now().instant) {
        run.typeahead.opening = None;
    }
    let mut kept_out = 0;
    while let Some(front) = run.typeahead.keys.front() {
        let (site, key) = (front.site.clone(), front.key);
        let now = focus(run);
        if site != now {
            if dialog(run, now.as_deref()) {
                run.typeahead.keys.pop_front();
                kept_out += 1;
            } else if let Some(front) = run.typeahead.keys.front_mut() {
                front.site = now;
            }
            continue;
        }
        if waiting(run, site.as_deref()) {
            break;
        }
        run.typeahead.keys.pop_front();
        run.behind = true;
        let effects = input::on_key(&mut run.ui, &run.session.tree, key, run.detection.now());
        run.apply(effects);
    }
    if kept_out > 0 {
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
