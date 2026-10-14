//! Discovery is shared; watcher scans leave project generations to explicit reload.
use super::*;
use bingo_plugin_host::discovery::Found;
pub(super) struct Directories {
    pub explicit: Vec<PathBuf>,
    pub layers: OnceLock<Vec<PathBuf>>,
}
impl Directories {
    pub fn new(explicit: Vec<PathBuf>) -> Self {
        Self {
            explicit,
            layers: OnceLock::new(),
        }
    }
    pub fn read(&self, project: bool, notices: &Notices, host: Option<&HostHandle>) -> Vec<Found> {
        let dirs = self.layers.get().cloned().unwrap_or_default();
        let dirs = if project {
            &dirs[..]
        } else {
            &dirs[..dirs.len().min(1)]
        };
        let authority = host.and_then(|host| host.service(&APPROVALS));
        discovery::approved(dirs, &self.explicit, notices, authority.as_deref())
    }
}
