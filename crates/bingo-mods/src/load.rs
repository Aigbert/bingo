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
    testing: bool,
}

impl Root {
    pub fn new(root: &Path, max_bytes: u64) -> std::io::Result<Self> {
        Ok(Self {
            root: root.canonicalize()?,
            max_bytes,
            testing: false,
        })
    }

    pub(crate) fn for_testing(mut self) -> Self {
        self.testing = true;
        self
    }

    pub fn source(&self, name: &str) -> Result<String, String> {
        if name == "bingo/testing" && self.testing {
            return crate::strip::strip(Path::new("testing.ts"), include_str!("../js/testing.ts"))
                .map_err(|e| e.to_string());
        }
        if name == "__bingo_ui__" {
            return crate::strip::strip(
                Path::new("ui.ts"),
                include_str!("../../../sdk/typescript/src/ui.ts"),
            )
            .map_err(|why| why.to_string());
        }
        if name == "__bingo_callbacks__" {
            return crate::strip::strip(
                Path::new("callbacks.ts"),
                include_str!("../../../sdk/typescript/src/callbacks.ts"),
            )
            .map_err(|why| why.to_string());
        }
        if name == "__bingo_dollar_types__" {
            return crate::strip::strip(
                Path::new("dollar-types.ts"),
                include_str!("../../../sdk/typescript/src/dollar-types.ts"),
            )
            .map_err(|why| why.to_string());
        }
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
            let state = crate::strip::strip(
                Path::new("state.ts"),
                include_str!("../../../sdk/typescript/src/state.ts"),
            )
            .map_err(|why| why.to_string())?;
            return Ok(format!(
                "export const h = globalThis.h; export const Fragment = globalThis.Fragment;\nexport {{ Handlers }} from './ui.ts';\nexport {{ Callbacks }} from './callbacks.ts';\n{hooks}\n{dollar}\n{state}"
            ));
        }
        let path = self.path(name)?;
        let mut source = String::new();
        std::fs::File::open(&path)
            .map_err(|e| e.to_string())?
            .take(self.max_bytes.saturating_add(1))
            .read_to_string(&mut source)
            .map_err(|e| e.to_string())?;
        if source.len() as u64 > self.max_bytes {
            return Err(format!("{name}:1: module exceeds the source limit"));
        }
        crate::strip::compile(
            path.strip_prefix(&self.root).map_err(|e| e.to_string())?,
            &source,
            self.testing,
        )
        .map_err(|why| why.to_string())
    }

    fn path(&self, name: &str) -> Result<PathBuf, String> {
        crate::imports::module_path(&self.root, Path::new(name))
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
        if base == "bingo" && name == "./ui.ts" {
            return Ok("__bingo_ui__".into());
        }
        if base == "bingo" && name == "./callbacks.ts" {
            return Ok("__bingo_callbacks__".into());
        }
        if base == "bingo" && name == "./dollar-types.ts" {
            return Ok("__bingo_dollar_types__".into());
        }
        if self.testing && name == "bingo/testing" {
            return Ok(name.into());
        }
        let path = crate::strip::resolve(Path::new(base), name)
            .map_err(|why| rquickjs::Exception::throw_type(ctx, &why))?;
        let name = path.to_string_lossy().replace('\\', "/");
        if name != "bingo" {
            let path = self
                .path(&name)
                .map_err(|why| rquickjs::Exception::throw_type(ctx, &why))?;
            return path
                .strip_prefix(&self.root)
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .map_err(|why| rquickjs::Exception::throw_type(ctx, &why.to_string()));
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
