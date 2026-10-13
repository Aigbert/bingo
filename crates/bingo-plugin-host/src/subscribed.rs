//! The state reads on which each drawing depends.
use crate::state::Read;
use bingo_sdk::SessionId;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Subscribed(BTreeMap<(SessionId, String), Vec<Read>>);
impl Subscribed {
    pub fn subscribe(&mut self, session: SessionId, request: String, reads: Vec<Read>) {
        self.0.insert((session, request), reads);
    }
    pub fn changed(&self, _scope: &str, _key: &str) -> BTreeMap<SessionId, Vec<String>> {
        BTreeMap::new()
    }
    pub fn forget(&mut self, session: &SessionId, request: &str) {
        self.0.remove(&(session.clone(), request.into()));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subscriptions_replace_reads_and_forget_without_crossing_sessions() {
        let mut subscriptions = Subscribed::default();
        let session = SessionId::from_raw("one");
        let read = |key: &str| Read {plugin:"counter".into(),scope:"global".into(),key:key.into(),version:None};
        subscriptions.subscribe(session.clone(), "pane".into(), vec![read("count")]);
        assert_eq!(subscriptions.changed("global", "count").get(&session), Some(&vec!["pane".to_string()]));
        assert!(subscriptions.changed("elsewhere", "count").is_empty());
        subscriptions.subscribe(session.clone(), "pane".into(), vec![read("name")]);
        assert!(subscriptions.changed("global", "count").is_empty());
        subscriptions.forget(&session, "pane");
        assert!(subscriptions.changed("global", "name").is_empty());
    }
}
