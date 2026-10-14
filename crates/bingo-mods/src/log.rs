//! Dynamic module targets share the application's existing tracing subscriber.
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};
use tracing::{
    Level, Metadata,
    callsite::{Callsite, Identifier},
    field::FieldSet,
    metadata::Kind,
    subscriber::Interest,
};
struct Site(OnceLock<&'static Metadata<'static>>);
static FALLBACK: Site = Site(OnceLock::new());
static META: Metadata<'static> = Metadata::new(
    "mod log",
    "mod",
    Level::INFO,
    None,
    None,
    None,
    FieldSet::new(&["message"], Identifier(&FALLBACK)),
    Kind::EVENT,
);
impl Callsite for Site {
    fn set_interest(&self, _: Interest) {}
    fn metadata(&self) -> &Metadata<'_> {
        self.0.get().copied().unwrap_or(&META)
    }
}
fn metadata(plugin: &str) -> &'static Metadata<'static> {
    static MODULES: Mutex<BTreeMap<String, &'static Metadata<'static>>> =
        Mutex::new(BTreeMap::new());
    let mut modules = MODULES.lock().unwrap_or_else(|error| error.into_inner());
    modules.entry(plugin.into()).or_insert_with(|| {
        let site = Box::leak(Box::new(Site(OnceLock::new())));
        let target = Box::leak(format!("mod::{plugin}").into_boxed_str());
        let metadata = Box::leak(Box::new(Metadata::new(
            "mod log",
            target,
            Level::INFO,
            None,
            None,
            None,
            FieldSet::new(&["message"], Identifier(site)),
            Kind::EVENT,
        )));
        let _ = site.0.set(metadata);
        tracing::callsite::register(site);
        metadata
    })
}
pub fn write(plugin: &str, text: &str) {
    let metadata = metadata(plugin);
    tracing::dispatcher::get_default(|dispatch| {
        if dispatch.enabled(metadata) {
            let values: &[Option<&dyn tracing::field::Value>] = &[Some(&text)];
            let fields = metadata.fields().value_set_all(values);
            dispatch.event(&tracing::Event::new(metadata, &fields));
        }
    });
}
#[cfg(test)]
mod tests {
    #[test]
    fn log_metadata_uses_one_attributed_target_per_module() {
        let one = super::metadata("example");
        assert_eq!(one.target(), "mod::example");
        assert!(std::ptr::eq(one, super::metadata("example")));
        assert_ne!(one.target(), super::metadata("other").target());
    }
}
