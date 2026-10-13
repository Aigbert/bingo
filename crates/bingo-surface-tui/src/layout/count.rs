//! Test instrumentation at the renderer entry points.

#[cfg(not(test))]
pub fn band(_: usize) {}
#[cfg(not(test))]
pub fn begin() {}
#[cfg(not(test))]
pub fn finish(_: &mut super::Layout) {}

#[cfg(test)]
pub use measured::*;

#[cfg(test)]
mod measured {
    use std::cell::Cell;
    thread_local! { static COUNTS: Cell<Option<[usize; 5]>> = const { Cell::new(None) }; }
    pub fn band(index: usize) {
        COUNTS.with(|counts| {
            if let Some(mut current) = counts.get() {
                current[index] += 1;
                counts.set(Some(current));
            }
        });
    }
    pub fn begin() {
        COUNTS.with(|counts| counts.set(Some([0; 5])));
    }
    pub fn finish(layout: &mut super::super::Layout) {
        layout.renders = COUNTS.with(|counts| counts.take().unwrap_or_default());
    }
}
