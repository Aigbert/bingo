//! Environment writes belong to the mod, never to the host process.
use super::{
    scope::{LIMIT, cap},
    world::{EnvGet, EnvSet},
};
use std::{collections::BTreeMap, sync::Mutex};
type Values = BTreeMap<(String, Option<bingo_sdk::ItemId>), BTreeMap<String, String>>;
#[derive(Default)]
pub struct Overlay(Mutex<Values>);
impl Overlay {
    pub fn get(&self, input: EnvGet) -> Result<Option<String>, String> {
        identifier(&input.name)?;
        match std::env::var(input.name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    pub fn set(&self, plugin: &str, input: EnvSet) -> Result<(), String> {
        identifier(&input.name)?;
        if input.value.contains('\0') {
            return Err("environment values may not contain NUL".into());
        }
        cap(input.value.len(), LIMIT).map_err(|e| e.to_string())?;
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry((plugin.into(), bingo_sdk::generation::current(plugin)))
            .or_default()
            .insert(input.name, input.value);
        Ok(())
    }
    pub fn values(&self, plugin: &str) -> BTreeMap<String, String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&(plugin.into(), bingo_sdk::generation::current(plugin)))
            .cloned()
            .unwrap_or_default()
    }
    pub fn remove(&self, plugin: &str, generation: &bingo_sdk::ItemId) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&(plugin.into(), Some(generation.clone())));
    }
}
fn identifier(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    if !chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        || !chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        return Err("environment name must be a literal identifier".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_identifiers_and_the_host_is_unchanged() {
        let overlay = Overlay::default();
        let name = format!("BINGO_WORLD_{}", std::process::id());
        let before = std::env::var(&name);
        overlay
            .set(
                "one",
                EnvSet {
                    session: bingo_sdk::SessionId::mint(),
                    name: name.clone(),
                    value: "child".into(),
                },
            )
            .unwrap();
        assert_eq!(std::env::var(&name), before);
        assert_eq!(overlay.values("one")[&name], "child");
        assert!(overlay.values("two").is_empty());
        for invalid in ["", "A=B", "1VAR", "a-b", "a\0b"] {
            assert!(identifier(invalid).is_err());
        }
    }
}
