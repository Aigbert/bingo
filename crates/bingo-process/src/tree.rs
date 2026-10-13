//! A child and the process tree whose lifetime it owns.

use std::io;
use std::process::ExitStatus;
use std::time::Duration;

#[cfg(windows)]
use process_wrap::tokio::JobObject;
#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout, Command};

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SpawnError(#[from] pub io::Error);

pub struct Tree(Box<dyn ChildWrapper>);

pub fn tree(command: Command) -> Result<Tree, SpawnError> {
    let mut wrapped = CommandWrap::from(command);
    #[cfg(unix)]
    wrapped.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    wrapped.wrap(JobObject);
    wrapped.wrap(KillOnDrop);
    Ok(Tree(wrapped.spawn()?))
}

impl Tree {
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.0.stdin().take()
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.0.stdout().take()
    }

    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.0.stderr().take()
    }

    pub fn id(&self) -> Option<u32> {
        self.0.id()
    }

    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.0.wait().await
    }

    pub fn start_kill(&mut self) -> io::Result<()> {
        self.0.start_kill()
    }

    /// Ask first on unix; Windows has no signal a program can answer.
    pub async fn terminate(&mut self, grace: Duration) -> io::Result<ExitStatus> {
        #[cfg(unix)]
        {
            let _ = self.0.signal(15);
            if let Ok(status) = tokio::time::timeout(grace, self.wait()).await {
                return status;
            }
        }
        #[cfg(windows)]
        let _ = grace;
        self.start_kill()?;
        self.wait().await
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        // A reaped child's group id may now belong to somebody else's work.
        if self.id().is_some() {
            let _ = self.start_kill();
        }
    }
}
