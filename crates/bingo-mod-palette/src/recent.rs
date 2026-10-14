use std::path::PathBuf;

pub struct Recent {
    path: PathBuf,
    names: Vec<String>,
}
impl Recent {
    pub fn load(path: PathBuf) -> Self {
        let names = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<String>>(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            names: names.into_iter().take(5).collect(),
        }
    }
    pub fn names(&self) -> &[String] {
        &self.names
    }
    pub fn remember(&mut self, line: &str) -> std::io::Result<()> {
        let Some(name) = line
            .strip_prefix('/')
            .and_then(|line| line.split_whitespace().next())
        else {
            return Ok(());
        };
        self.names.retain(|old| old != name);
        self.names.insert(0, name.into());
        self.names.truncate(5);
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_vec(&self.names)?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_distinct_names_survive_reopening_without_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("palette/recent.json");
        let mut recent = Recent::load(path.clone());
        for name in [
            "/one secret",
            "/two",
            "/three",
            "/four",
            "/five",
            "/six",
            "/two",
        ] {
            recent.remember(name).unwrap();
        }
        assert_eq!(
            Recent::load(path).names(),
            ["two", "six", "five", "four", "three"]
        );
    }
}
