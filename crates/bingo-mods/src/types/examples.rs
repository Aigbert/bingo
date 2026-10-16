//! The worked examples, laid under `.bingo/examples/<name>/` so an author
//! starts from a mod that validates and passes its tests. The repository's
//! `examples/plugins/` is the one copy; these are its bytes.

/// An example's laid files, as `(path inside the example, text)`.
pub(super) struct Example {
    pub(super) name: &'static str,
    pub(super) files: &'static [(&'static str, &'static str)],
}

macro_rules! example {
    ($name:literal: $($file:literal),+ $(,)?) => {
        Example {
            name: $name,
            files: &[$((
                $file,
                include_str!(concat!("../../../../examples/plugins/", $name, "/", $file)),
            )),+],
        }
    };
}

pub(super) const EXAMPLES: &[Example] = &[
    example!("tool-call": "README.md", "plugin.json", "register.ts", "register.test.ts", "types/index.d.ts"),
    example!("band": "README.md", "plugin.json", "register.ts", "register.test.ts", "types/index.d.ts"),
    example!("pane": "README.md", "plugin.json", "register.ts", "register.test.ts", "types/index.d.ts"),
    example!("ts-status": "README.md", "plugin.json", "hooks/register.ts", "hooks/register.test.ts", "types/index.d.ts"),
];

/// Every laid example file, as its path under `.bingo/`.
pub(super) fn laid() -> impl Iterator<Item = (String, &'static str)> {
    EXAMPLES.iter().flat_map(|example| {
        example
            .files
            .iter()
            .map(|(path, text)| (format!("examples/{}/{path}", example.name), *text))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Files an example keeps in the repository but does not lay, and why:
    /// `tsconfig.json` is what `bingo plugins types` writes where the example
    /// is copied to; the parity files and `ts-status`'s process
    /// entry reach into this repository.
    const KEPT_HOME: &[&str] = &[
        "tsconfig.json",
        "parity-entry.ts",
        "register.parity.test.ts",
        "main.ts",
    ];

    fn walk(root: &Path, directory: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
            {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, found);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    fn kept_home(path: &str) -> bool {
        KEPT_HOME.contains(&path.rsplit('/').next().unwrap_or(path))
    }

    #[test]
    fn every_example_file_is_laid_or_kept_home_for_a_reason() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins");
        for example in EXAMPLES {
            let root = examples.join(example.name);
            let mut found = Vec::new();
            walk(&root, &root, &mut found);
            let laid: Vec<&str> = example.files.iter().map(|(path, _)| *path).collect();
            for path in &found {
                assert!(
                    laid.contains(&path.as_str()) ^ kept_home(path),
                    "{}/{path} must be laid or kept home, not both",
                    example.name
                );
            }
        }
    }

    #[test]
    fn laid_examples_name_no_repository_path() {
        for (path, text) in laid() {
            assert!(!text.contains("examples/plugins"), "{path}");
            assert!(!text.contains("sdk/typescript"), "{path}");
        }
    }
}
