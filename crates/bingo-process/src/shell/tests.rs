use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Env {
    vars: BTreeMap<String, OsString>,
    paths: Vec<PathBuf>,
    files: Vec<PathBuf>,
}

impl Env {
    fn with_files(files: &[&str]) -> Self {
        Self {
            files: files.iter().map(PathBuf::from).collect(),
            ..Self::default()
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        self.vars.insert(key.into(), value.into());
    }
}

impl ShellEnv for Env {
    fn var(&self, name: &str) -> Option<OsString> {
        self.vars.get(name).cloned()
    }
    fn path(&self) -> Vec<PathBuf> {
        self.paths.clone()
    }
    fn is_file(&self, path: &Path) -> bool {
        self.files.contains(&path.to_path_buf())
    }
}

fn selected(env: &Env, platform: Platform, expected: &str) {
    assert_eq!(
        resolve_with(env, platform).expect("shell resolved").path(),
        Path::new(expected)
    );
}

#[test]
fn an_absolute_override_wins_on_either_platform() {
    for (platform, path) in [
        (Platform::Unix, "/custom/bash"),
        (Platform::Windows, "D:/My Shell/bash.exe"),
    ] {
        let mut env = Env::with_files(&[path, "/bin/bash", "C:/Git/bin/bash.exe"]);
        env.set("BINGO_SHELL", path);
        selected(&env, platform, path);
    }
}

#[test]
fn a_missing_override_never_falls_through() {
    for platform in [Platform::Unix, Platform::Windows] {
        let mut env = Env::with_files(&["/bin/bash", "C:/Git/bin/bash.exe"]);
        env.set("BINGO_SHELL", "/nonexistent");
        let error = resolve_with(&env, platform).expect_err("override missing");
        assert_eq!(error.tried, [PathBuf::from("/nonexistent")]);
        assert!(
            error
                .to_string()
                .contains("Install Git for Windows or set BINGO_SHELL")
        );
    }
}

#[test]
fn relative_and_empty_overrides_are_rejected_even_if_present() {
    for platform in [Platform::Unix, Platform::Windows] {
        for path in ["bash", ""] {
            let mut env = Env::with_files(&[path]);
            env.set("BINGO_SHELL", path);
            assert!(resolve_with(&env, platform).is_err());
        }
    }
}

#[test]
fn unix_prefers_bash_then_sh_and_reports_both_when_missing() {
    selected(
        &Env::with_files(&["/bin/bash", "/bin/sh"]),
        Platform::Unix,
        "/bin/bash",
    );
    selected(&Env::with_files(&["/bin/sh"]), Platform::Unix, "/bin/sh");
    assert_eq!(
        resolve_with(&Env::default(), Platform::Unix)
            .unwrap_err()
            .tried,
        [PathBuf::from("/bin/bash"), PathBuf::from("/bin/sh")]
    );
}

#[test]
fn windows_prefers_the_first_safe_path_shell() {
    let mut env = Env::with_files(&["D:/MSYS/bin/bash.exe", "C:/Git/bin/bash.exe"]);
    env.paths = ["D:/MSYS/bin", "C:/Git/bin"].map(PathBuf::from).into();
    selected(&env, Platform::Windows, "D:/MSYS/bin/bash.exe");
}

#[test]
fn windows_skips_wsl_case_insensitively_and_by_directory_boundary() {
    let mut env = Env::with_files(&[
        "c:/WINDOWS/System32/bash.exe",
        "C:/Users/me/windowsAPPS/bash.exe",
        "C:/WindowsExtra/bash.exe",
    ]);
    env.set("SystemRoot", r"C:\Windows");
    env.paths = [
        r"c:\WINDOWS\System32",
        r"C:\Users\me\windowsAPPS",
        "C:/WindowsExtra",
    ]
    .map(PathBuf::from)
    .into();
    selected(&env, Platform::Windows, "C:/WindowsExtra/bash.exe");
}

fn installations() -> Env {
    let mut env = Env::with_files(&[
        "C:/PF/Git/bin/bash.exe",
        "C:/PF86/Git/bin/bash.exe",
        "C:/Local/Programs/Git/bin/bash.exe",
        "D:/Portable/cmd/git.exe",
        "D:/Portable/bin/bash.exe",
    ]);
    env.set("ProgramFiles", r"C:\PF");
    env.set("ProgramFiles(x86)", "C:/PF86");
    env.set("LocalAppData", "C:/Local");
    env.paths = vec![PathBuf::from("D:/Portable/cmd")];
    env
}

#[test]
fn windows_tries_all_git_installations_before_deriving_from_git() {
    let mut env = installations();
    for expected in [
        "C:/PF/Git/bin/bash.exe",
        "C:/PF86/Git/bin/bash.exe",
        "C:/Local/Programs/Git/bin/bash.exe",
        "D:/Portable/bin/bash.exe",
    ] {
        selected(&env, Platform::Windows, expected);
        env.files.retain(|path| path != Path::new(expected));
    }
    let tried = resolve_with(&env, Platform::Windows).unwrap_err().tried;
    assert_eq!(
        tried,
        [
            "D:/Portable/cmd/bash.exe",
            "C:/PF/Git/bin/bash.exe",
            "C:/PF86/Git/bin/bash.exe",
            "C:/Local/Programs/Git/bin/bash.exe",
            "D:/Portable/bin/bash.exe"
        ]
        .map(PathBuf::from)
    );
}

#[test]
fn deriving_a_shell_requires_git_to_exist() {
    let mut env = Env::with_files(&["D:/Portable/bin/bash.exe"]);
    env.paths = vec![PathBuf::from("D:/Portable/cmd")];
    assert!(resolve_with(&env, Platform::Windows).is_err());
}
