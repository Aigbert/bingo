//! Project precedence changes only on an explicit scan, including process plugins.
use super::*;
use bingo_plugin_host::discovery::Found;

pub(super) struct Directories {
    pub explicit: Vec<PathBuf>,
    pub layers: OnceLock<Vec<PathBuf>>,
    project_names: Mutex<BTreeSet<String>>,
}
impl Directories {
    pub fn new(explicit: Vec<PathBuf>) -> Self {
        Self {
            explicit,
            layers: OnceLock::new(),
            project_names: Mutex::default(),
        }
    }
    pub fn read(&self, project: bool, notices: &Notices) -> Vec<Found> {
        let dirs = self.layers.get().cloned().unwrap_or_default();
        let mut found: BTreeMap<_, _> = if project {
            let found = discovery::discover(&dirs, notices);
            *self
                .project_names
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = found
                .iter()
                .filter(|found| found.origin == PluginOrigin::Project)
                .map(|found| found.name.clone())
                .collect();
            found
                .into_iter()
                .map(|found| (found.name.clone(), found))
                .collect()
        } else {
            let names = self
                .project_names
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            discovery::discover(&dirs[..dirs.len().min(1)], notices)
                .into_iter()
                .filter(|found| !names.contains(&found.name))
                .map(|found| (found.name.clone(), found))
                .collect()
        };
        for root in &self.explicit {
            if let Some(plugin) = discovery::read(root, notices) {
                found.insert(plugin.name.clone(), plugin);
            }
        }
        found.into_values().collect()
    }
}
