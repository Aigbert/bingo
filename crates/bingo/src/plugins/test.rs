//! A disposable real host and a fresh QuickJS test runtime for each file.
mod driver;
mod mounts;
use bingo_mods::testing::{Outcome, Report, Runner};
use bingo_sdk::KernelError;
use std::path::{Path, PathBuf};

pub async fn run(root: &Path, json: bool) -> Result<(), KernelError> {
    let root = root.canonicalize().map_err(super::invalid)?;
    let manifest = bingo_plugin_rpc::validate::read(&root)?;
    if !matches!(
        manifest.launch,
        bingo_plugin_host::manifest::Launch::Module(_)
    ) {
        return Err(super::invalid("plugins test requires a module manifest"));
    }
    let files = files(&root).map_err(super::invalid)?;
    if files.is_empty() {
        return Err(super::invalid("no *.test.ts or *.test.tsx files found"));
    }
    let reports = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        runtime.block_on(execute(&root, &manifest.name, files))
    })
    .await
    .map_err(super::invalid)?
    .map_err(super::invalid)?;
    let failed = reports
        .iter()
        .any(|report| matches!(report.outcome, Outcome::Fail { .. }));
    for report in reports {
        print(&report, json)?;
    }
    if failed {
        Err(super::invalid("module tests failed"))
    } else {
        Ok(())
    }
}
fn files(root: &Path) -> Result<Vec<String>, String> {
    let mut directories = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() && !matches!(name.as_str(), ".bingo" | "node_modules" | ".git") {
                directories.push(entry.path());
            } else if kind.is_file() && (name.ends_with(".test.ts") || name.ends_with(".test.tsx"))
            {
                files.push(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    files.sort();
    Ok(files)
}
async fn execute(root: &Path, name: &str, files: Vec<String>) -> Result<Vec<Report>, String> {
    let mut reports = Vec::new();
    for file in files {
        let runner = Runner::load(root, &file, Default::default()).await?;
        let mut driver = driver::Driver::new(root, name).await?;
        let result = runner.run(&mut driver, serde_json::json!({})).await;
        runner.close().await;
        driver.close().await;
        reports.extend(result?);
    }
    Ok(reports)
}
fn print(report: &Report, json: bool) -> Result<(), KernelError> {
    if json {
        println!("{}", serde_json::to_string(report).map_err(super::invalid)?);
    } else {
        match &report.outcome {
            Outcome::Pass => println!("PASS {} {}", report.file, report.test),
            Outcome::Skip => println!("SKIP {} {}", report.file, report.test),
            Outcome::Fail { message, at } => {
                println!("FAIL {} {}: {message} ({at})", report.file, report.test)
            }
        }
    }
    Ok(())
}
