//! The memory owner a scene's world keeps: the prototype's two instruction
//! files and three memories, and `/memory` as bingo-context lists it.
use super::kernel::Kernel;
use crate::test_support::ember::world::rows;
use bingo_sdk::*;
use bingo_services::memory::{self, Forget, Kind, List, Listing, Loaded, MemoryRow, Read, Scope};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(super) fn register(kernel: &Kernel, home: PathBuf) {
    kernel.add(Arc::new(Command));
    let owner: Arc<dyn memory::Service> = Arc::new(Owner {
        home,
        forgotten: Mutex::default(),
    });
    super::builtins::serve(kernel, memory::KEY.name(), owner);
}

/// The world's memories, less the ones the person forgot.
struct Owner {
    home: PathBuf,
    forgotten: Mutex<Vec<String>>,
}

impl Owner {
    /// The project's file sits in the session's folder, the person's own in
    /// the folder bingo keeps under their home; the prototype's note says
    /// where it speaks and how many lines it has.
    fn instruction(&self, name: &str, note: &str, cwd: &std::path::Path) -> Loaded {
        let (path, scope) = match name.strip_prefix("~/") {
            Some(own) => (self.home.join(own), Scope::User),
            None => (cwd.join(name), Scope::Project),
        };
        let lines = note
            .rsplit(" · ")
            .next()
            .and_then(|count| count.trim_end_matches(" lines").parse().ok())
            .unwrap();
        Loaded {
            path,
            tokens: 0,
            scope,
            lines,
        }
    }

    /// A memory says what it keeps in its name and is dated by the
    /// prototype's day; the world knows no description of it.
    fn memory(&self, name: &str, day: &str, cwd: &std::path::Path) -> MemoryRow {
        MemoryRow {
            scope: Scope::Project,
            name: name.into(),
            kind: Kind::Project,
            description: String::new(),
            path: cwd.join(".bingo/memory").join(format!("{name}.md")),
            tokens: 0,
            error: None,
            written: jiff::civil::Date::strptime("%b %d %Y", format!("{day} 2026")).ok(),
        }
    }
}

#[async_trait::async_trait]
impl memory::Service for Owner {
    async fn list(&self, input: List) -> Result<Listing, KernelError> {
        let forgotten = self.forgotten.lock().unwrap().clone();
        let mut listing = Listing::default();
        for row in rows("memory") {
            let (name, note) = (row["t"].as_str().unwrap(), row["d"].as_str().unwrap());
            if row["kind"] == "instructions" {
                listing
                    .instructions
                    .push(self.instruction(name, note, &input.cwd));
            } else if !forgotten.iter().any(|gone| gone == name) {
                listing.memories.push(self.memory(name, note, &input.cwd));
            }
        }
        Ok(listing)
    }
    async fn forget(&self, input: Forget) -> Result<String, KernelError> {
        self.forgotten.lock().unwrap().push(input.name.clone());
        Ok(format!("forgot {}", input.name))
    }
    async fn read(&self, _: Read) -> Result<String, KernelError> {
        Err(KernelError::new(
            ErrorCode::InvalidInput,
            "a scene reads no memory",
        ))
    }
}

/// `/memory` as bingo-context lists it; a surface that shows sheets is
/// answered by the memory owner first.
struct Command;

#[async_trait::async_trait]
impl bingo_sdk::Command for Command {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            keys: vec![],
            hidden: false,
            opens: false,
            name: "memory".into(),
            aliases: vec![],
            hint: "read and forget memories and instructions".into(),
            args: ArgSpec::None,
            instant: true,
            family: "memory".into(),
        }
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::Applied { message: None })
    }
}
