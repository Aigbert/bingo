//! Skills as `/name` commands. The source answers from the library, which is
//! why a skill saved mid-session is in the next completion (ADR-0009 §1).

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    ArgSpec, Command, CommandContext, CommandOutcome, CommandSource, CommandSpec, KernelError,
};

use crate::expand::expand;
use crate::library::Library;
use crate::shelf::Shelf;
use crate::skill::Skill;

/// One `/name` per skill.
#[derive(Debug)]
pub struct SkillCommands {
    library: Arc<Library>,
}

impl SkillCommands {
    pub fn new(library: Arc<Library>) -> Self {
        Self { library }
    }
}

#[async_trait]
impl CommandSource for SkillCommands {
    fn id(&self) -> &str {
        "skills"
    }

    async fn commands(&self, cwd: &Path) -> Vec<Arc<dyn Command>> {
        self.library
            .skills(cwd)
            .iter()
            .map(|skill| {
                let shelf = self.library.shelf(skill);
                Arc::new(SkillCommand::new(skill.clone(), shelf)) as Arc<dyn Command>
            })
            .collect()
    }
}

/// One skill, typed as `/name args`, and the shelf it was read from.
#[derive(Debug)]
pub struct SkillCommand {
    skill: Skill,
    shelf: Shelf,
}

impl SkillCommand {
    pub fn new(skill: Skill, shelf: Shelf) -> Self {
        Self { skill, shelf }
    }
}

#[async_trait]
impl Command for SkillCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: self.skill.name.clone(),
            aliases: Vec::new(),
            // What the skill is for: the menu's line. The argument syntax is
            // `args`'s to say, and a client completing it reads it there.
            hint: crate::listing::one_line(&self.skill.description),
            args: ArgSpec::Free {
                hint: self.skill.argument_hint.clone().unwrap_or_default(),
            },
            // A skill is a prompt: it opens a turn, so it waits for the one
            // that is running.
            instant: false,
            family: self.shelf.family(),
        }
    }

    async fn run(&self, args: &str, _cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::Prompt {
            text: expand(&self.skill, args),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::tests::{Tree, command_context};

    fn command(source: &str) -> SkillCommand {
        SkillCommand::new(
            Skill::parse("deploy", PathBuf::from("/skills/deploy"), source),
            Shelf::Project,
        )
    }

    async fn run(command: &SkillCommand, args: &str) -> CommandOutcome {
        command
            .run(args, &command_context())
            .await
            .expect("a skill command never fails")
    }

    #[tokio::test]
    async fn a_skill_command_becomes_the_prompt_its_body_spells() {
        let command = command("---\ndescription: Ship it\n---\nDeploy $1 now.\n");
        assert_eq!(
            run(&command, "staging").await,
            CommandOutcome::Prompt {
                text: "Base directory for this skill: /skills/deploy\n\nDeploy staging now.\n"
                    .into()
            },
            "the command path expands like the tool path, base line and all"
        );
    }

    #[test]
    fn the_spec_waits_for_the_turn_and_takes_free_text() {
        let spec =
            command("---\ndescription: Ship it\nargument-hint: \"[env]\"\n---\nbody\n").spec();
        assert_eq!(spec.name, "deploy");
        assert_eq!(spec.hint, "Ship it", "the menu's line is what it is for");
        assert_eq!(spec.family, "skill/project");
        assert!(!spec.instant, "a skill opens a turn");
        assert_eq!(
            spec.args,
            ArgSpec::Free {
                hint: "[env]".into()
            }
        );
    }

    #[test]
    fn a_skill_that_declared_no_hint_completes_free_text_with_no_hint() {
        let spec = command("---\ndescription: Ship the build\n---\nbody\n").spec();
        assert_eq!(spec.hint, "Ship the build");
        assert_eq!(
            spec.args,
            ArgSpec::Free {
                hint: String::new()
            }
        );
    }

    #[tokio::test]
    async fn the_source_mints_one_command_per_skill() {
        let tree = Tree::new();
        tree.user_skill("alpha", "---\ndescription: a\n---\na\n");
        tree.user_skill("beta", "---\ndescription: b\n---\nb\n");
        let source =
            SkillCommands::new(Arc::new(Library::new(bingo_sdk::Env::rooted(tree.root()))));

        // The project layers come from the process's own directory, so assert
        // what this tree put there rather than the whole list.
        let names: Vec<String> = source
            .commands(&tree.cwd())
            .await
            .iter()
            .map(|command| command.spec().name)
            .collect();
        for expected in ["alpha", "beta", "guide"] {
            assert!(names.iter().any(|name| name == expected), "{names:?}");
        }
        let mut once = names.clone();
        once.sort();
        once.dedup();
        assert_eq!(once.len(), names.len(), "one command per name: {names:?}");
        assert_eq!(source.id(), "skills");
    }

    #[tokio::test]
    async fn each_command_names_the_shelf_its_skill_was_read_from() {
        let tree = Tree::new();
        tree.user_skill("mine", "mine\n");
        let cwd = tree.project_skill("work", "theirs", "theirs\n");
        let library = Arc::new(Library::new(bingo_sdk::Env::rooted(tree.root())));
        library
            .read_pages(&crate::tests::host_with_pages(&[(
                "bingo.mcp",
                crate::tests::PAGES,
            )]))
            .await;

        let families: std::collections::BTreeMap<String, String> = SkillCommands::new(library)
            .commands(&cwd)
            .await
            .iter()
            .map(|command| (command.spec().name, command.spec().family))
            .collect();
        for (name, family) in [
            ("mine", "skill/yours"),
            ("theirs", "skill/project"),
            ("guide", "skill/built in"),
            ("guide-first", "skill/guide"),
        ] {
            assert_eq!(families[name], family, "{name}");
        }
    }
}
