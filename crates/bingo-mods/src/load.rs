//! Resolve and load only the root's files and the bundled bingo module.
use std::io::Read;
use std::path::{Path, PathBuf};

use rquickjs::{
    Ctx, Module,
    loader::{ImportAttributes, Loader, Resolver},
    module::Declared,
};

#[derive(Clone)]
pub struct Root {
    root: PathBuf,
    max_bytes: u64,
}

impl Root {
    pub fn new(root: &Path, max_bytes: u64) -> std::io::Result<Self> {
        Ok(Self {
            root: root.canonicalize()?,
            max_bytes,
        })
    }

    pub fn source(&self, name: &str) -> Result<String, String> {
        if name == "bingo" {
            let hooks = crate::strip::strip(
                Path::new("on.ts"),
                include_str!("../../../sdk/typescript/src/on.ts"),
            )
            .map_err(|why| why.to_string())?;
            let dollar = crate::strip::strip(
                Path::new("dollar.ts"),
                include_str!("../../../sdk/typescript/src/dollar.ts"),
            )
            .map_err(|why| why.to_string())?;
            return Ok(format!("{hooks}\n{dollar}"));
        }
        let path = self.path(name)?;
        let mut source = String::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(self.max_bytes.saturating_add(1))
            .read_to_string(&mut source)
            .map_err(|e| e.to_string())?;
        if source.len() as u64 > self.max_bytes {
            return Err(format!("{name}:1: module exceeds the source limit"));
        }
        crate::strip::strip(Path::new(name), &source).map_err(|why| why.to_string())
    }

    fn path(&self, name: &str) -> Result<PathBuf, String> {
        let path = self
            .root
            .join(name)
            .canonicalize()
            .map_err(|e| format!("{name}:1: {e}"))?;
        if !path.starts_with(&self.root) {
            return Err(format!("{name}:1: module escapes the mod root"));
        }
        Ok(path)
    }
}

impl Resolver for Root {
    fn resolve<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        let path = crate::strip::resolve(Path::new(base), name)
            .map_err(|why| rquickjs::Exception::throw_type(ctx, &why))?;
        let name = path.to_string_lossy().replace('\\', "/");
        if name != "bingo" {
            self.path(&name)
                .map_err(|why| rquickjs::Exception::throw_type(ctx, &why))?;
        }
        Ok(name)
    }
}

impl Loader for Root {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        let source = self
            .source(name)
            .map_err(|why| rquickjs::Exception::throw_type(ctx, &why))?;
        Module::declare(ctx.clone(), name, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_imports_resolve_inside_the_root() {
        assert_eq!(
            crate::strip::resolve(Path::new("nested/register.ts"), "../helper.ts").unwrap(),
            Path::new("helper.ts")
        );
        assert!(crate::strip::resolve(Path::new("nested/register.ts"), "../../escape.ts").is_err());
    }
    #[test]
    fn an_absolute_path_cannot_load_an_outside_file_on_any_platform() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        assert!(
            Root::new(root.path(), 4096)
                .unwrap()
                .source(outside.path().to_str().unwrap())
                .unwrap_err()
                .contains("escapes")
        );
    }
    #[cfg(unix)]
    #[test]
    fn a_symlink_cannot_import_an_outside_file() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape.ts")).unwrap();
        assert!(
            Root::new(root.path(), 4096)
                .unwrap()
                .source("escape.ts")
                .unwrap_err()
                .contains("escapes")
        );
    }
}
