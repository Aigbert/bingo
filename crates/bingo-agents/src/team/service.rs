//! The team file owner implements the shared typed contract.
use std::path::Path;
use async_trait::async_trait;
use bingo_services::team::{self, TeamError};
use serde_json::Value;
use crate::team::file;

#[derive(Debug, Default, Clone, Copy)]
pub struct TeamFile;

#[async_trait]
impl team::TeamFile for TeamFile {
    async fn section(&self, cwd: &Path, key: &str) -> Result<Value, TeamError> {
        file::section(cwd, key).map(|section| section.unwrap_or(Value::Null)).map_err(|error| TeamError(error.message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::ServiceError;
    use serde_json::json;
    const SECTION: &str = "section";
    fn wire() -> std::sync::Arc<dyn bingo_sdk::WireService> {
        bingo_services::team::wire(std::sync::Arc::new(TeamFile))
    }
    use crate::tests::Tree;

    const FULL: &str = r#"{
        "roles": [{ "name": "reviewer" }],
        "rooms": [{ "name": "design", "members": ["reviewer"] }]
    }"#;

    async fn asked(cwd: &std::path::Path, key: &str) -> Result<Value, ServiceError> {
        wire()
            .call(SECTION, json!({ "cwd": cwd, "key": key }))
            .await
    }

    /// The recorded answer, which is another plugin's contract with this one:
    /// the key's value exactly as a person wrote it, under the method's name.
    #[tokio::test]
    async fn the_service_answers_one_key_as_a_person_wrote_it() {
        let tree = Tree::new();
        let cwd = tree.team("work", FULL);
        assert_eq!(
            asked(&cwd, "rooms").await.expect("it answers"),
            json!({ "section": [{ "name": "design", "members": ["reviewer"] }] })
        );
        assert_eq!(
            asked(&cwd, "roles").await.expect("it answers"),
            json!({ "section": [{ "name": "reviewer" }] }),
            "this plugin's own keys are not privileged"
        );
    }

    /// A project that declared nothing under that name, and one that declared
    /// no team at all, say the same thing: there is nothing there.
    #[tokio::test]
    async fn a_key_nobody_wrote_and_a_file_nobody_wrote_answer_alike() {
        let tree = Tree::new();
        let cwd = tree.team("work", r#"{ "roles": [] }"#);
        assert_eq!(
            asked(&cwd, "rooms").await.expect("it answers"),
            json!({ "section": null })
        );
        let bare = Tree::new();
        assert_eq!(
            asked(&bare.cwd(), "rooms").await.expect("it answers"),
            json!({ "section": null })
        );
    }

    /// A file that will not parse is one mistake reported once, by the plugin
    /// that owns the file, naming it.
    #[tokio::test]
    async fn a_file_that_will_not_parse_says_which_one() {
        let tree = Tree::new();
        let cwd = tree.team("work", "{ not json");
        let refused = asked(&cwd, "rooms").await.expect_err("a refusal");
        assert!(refused.to_string().contains("team.json"), "{refused}");
    }

    #[tokio::test]
    async fn a_method_this_service_does_not_speak_says_which_one_it_does() {
        let refused = wire()
            .call("roles", json!({}))
            .await
            .expect_err("a refusal");
        assert!(refused.to_string().contains(SECTION), "{refused}");
    }

    #[tokio::test]
    async fn params_that_name_no_directory_are_refused_rather_than_guessed() {
        assert!(
            wire()
                .call(SECTION, json!({ "key": "rooms" }))
                .await
                .is_err()
        );
    }
}
