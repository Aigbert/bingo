//! The readable entries and the files that could not be read.

#[derive(Debug)]
pub struct Loaded<T> {
    pub entries: Vec<T>,
    pub unreadable: Vec<Unreadable>,
}

impl<T> Default for Loaded<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            unreadable: Vec::new(),
        }
    }
}

impl<T> Loaded<T> {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Unreadable {
    pub file: String,
    pub why: String,
}
