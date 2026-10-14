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
            names: names
                .into_iter()
                .filter_map(|line| {
                    line.trim_start_matches('/')
                        .split_whitespace()
                        .next()
                        .map(str::to_owned)
                })
                .take(5)
                .collect(),
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

#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn provider_and_login_arguments_never_reach_the_recent_file() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("recent.json");
        let mut recent = Recent::load(path.clone());
        for line in [
            "/provider add private --key provider-secret",
            "/login pasted-login-secret",
        ] {
            recent.remember(line).unwrap();
            let disk = std::fs::read_to_string(&path).unwrap();
            assert!(!disk.contains("secret"), "{disk}");
        }
        assert_eq!(Recent::load(path).names(), ["login", "provider"]);
    }
    #[test]
    fn loaded_entries_are_normalized_before_the_next_write() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("recent.json");
        std::fs::write(
            &path,
            r#"["/provider add --key old-secret","login pasted-secret"]"#,
        )
        .unwrap();
        let mut recent = Recent::load(path.clone());
        recent.remember("/status").unwrap();
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            r#"["status","provider","login"]"#
        );
    }
}
