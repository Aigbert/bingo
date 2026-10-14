use super::*;

pub(super) struct Suite {
    pub module: PathBuf,
    pub name: String,
    temporary: Option<tempfile::TempDir>,
}
impl Suite {
    pub fn read(root: &Path) -> Result<Self, String> {
        if root
            .join("plugin.json")
            .try_exists()
            .map_err(driver::error)?
        {
            let manifest = bingo_plugin_rpc::validate::read(root).map_err(driver::error)?;
            if !matches!(
                manifest.launch,
                bingo_plugin_host::manifest::Launch::Module(_)
            ) {
                return Err("plugins test requires a module manifest".into());
            }
            return Ok(Self {
                module: root.into(),
                name: manifest.name,
                temporary: None,
            });
        }
        if !root.join("package.json").is_file() {
            return Err("plugins test requires a module manifest or a library package.json".into());
        }
        let temporary = tempfile::tempdir().map_err(driver::error)?;
        let module = temporary.path().join("kit-library");
        std::fs::create_dir(&module).map_err(driver::error)?;
        std::fs::write(
            module.join("plugin.json"),
            r#"{"name":"kit-library","version":"1","module":"register.ts"}"#,
        )
        .map_err(driver::error)?;
        std::fs::write(
            module.join("register.ts"),
            "export const register = () => {};",
        )
        .map_err(driver::error)?;
        Ok(Self {
            module,
            name: "kit-library".into(),
            temporary: Some(temporary),
        })
    }
    pub fn select(
        &self,
        root: &Path,
        files: Vec<String>,
    ) -> Result<(Vec<String>, Vec<Report>), String> {
        let mut runnable = Vec::new();
        let mut skipped = Vec::new();
        for file in files {
            if self.temporary.is_some() && !bingo_mods::testing::imports_testing(&root.join(&file))?
            {
                skipped.push(Report {
                    file,
                    test: "uses another test runner; run the package test script".into(),
                    outcome: Outcome::Skip,
                });
            } else {
                runnable.push(file);
            }
        }
        Ok((runnable, skipped))
    }
}
