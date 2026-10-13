//! Only drawn requests have a memo; late replies cannot replace newer inputs.
use bingo_sdk::{SessionId, SessionState, View, render::{RenderRequest, SurfaceName}, slot::Shown};

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub session: SessionId,
    pub request: RenderRequest,
    generation: u64,
}
impl Key {
    pub fn new(state: &SessionState, request: &RenderRequest, columns: u16, rows: u16) -> Self {
        let mut request = request.clone();
        request.surface = SurfaceName::Terminal;
        request.viewport.columns = columns;
        request.viewport.rows = rows;
        Self {session: state.summary.id.clone(), generation: state.invalidated.get(&request.request_id).copied().unwrap_or_default(), request}
    }
    fn same(&self, other: &Self) -> bool { self == other }
}
#[derive(Debug, Default)]
pub struct Memo { entries: Vec<(Key, Option<View>)> }
impl Memo {
    pub fn request(&mut self, _key: &Key) -> bool { false }
    pub fn answer(&mut self, _key: Key, _view: View) {}
    pub fn view<'a>(&'a self, shown: &'a Shown, state: &SessionState, columns: u16) -> Option<&'a View> {
        match shown {
            Shown::View(view) => Some(view),
            Shown::Pull{request} => {let key=Key::new(state,request,columns,0);self.entries.iter().find(|(one,_)|one.same(&key)).and_then(|(_,view)|view.as_ref())}
        }
    }
    pub fn retain(&mut self, _drawn: &[Key]) {}
}
#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::render::{Component, Viewport};
    #[test]
    fn width_props_and_generation_replace_answers_but_height_does_not() {
        let mut state=crate::test_support::state();
        let request=RenderRequest {component:Component::Pane,request_id:"counter".into(),surface:SurfaceName::Terminal,props:serde_json::json!({}),viewport:Viewport{columns:0,rows:0}};
        let mut memo=Memo::default();
        let first=Key::new(&state,&request,80,8);
        assert!(memo.request(&first));
        assert!(!memo.request(&first));
        memo.answer(first.clone(),View::text("old"));
        assert!(!memo.request(&Key::new(&state,&request,80,12)));
        state.invalidated.insert("counter".into(),1);
        let next=Key::new(&state,&request,80,8);
        assert!(memo.request(&next));
        memo.answer(first,View::text("stale"));
        assert!(memo.view(&Shown::Pull{request:request.clone()},&state,80).is_none());
        memo.answer(next,View::text("new"));
        assert_eq!(memo.view(&Shown::Pull{request:request.clone()},&state,80).unwrap().fold(),"new");
        assert!(memo.request(&Key::new(&state,&request,200,8)));
        memo.retain(&[]);
        assert!(memo.entries.is_empty());
    }
}
