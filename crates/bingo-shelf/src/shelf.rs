//! The directory operations shared by all codecs.

use std::io;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::{Codec, Loaded, Unreadable, id};

#[derive(Debug)]
pub struct Shelf<C> {
    dir: PathBuf,
    codec: PhantomData<C>,
}

impl<C: Codec> Shelf<C> {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            codec: PhantomData,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{}", C::EXT))
    }

    pub fn load(&self) -> Loaded<C::Record> {
        let Ok(files) = std::fs::read_dir(&self.dir) else {
            return Loaded::default();
        };
        let mut paths: Vec<_> = files
            .flatten()
            .map(|file| file.path())
            .filter(|path| candidate::<C>(path))
            .collect();
        // Every codec receives the filename stem as its id.
        paths.sort_by(|a, b| a.file_stem().cmp(&b.file_stem()));
        let mut loaded = Loaded::default();
        for path in paths {
            match read::<C>(&path) {
                Ok(record) => loaded.entries.push(record),
                Err(why) => loaded.unreadable.push(Unreadable {
                    file: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    why,
                }),
            }
        }
        loaded.unreadable.sort_by(|a, b| a.file.cmp(&b.file));
        loaded
    }

    pub fn save(&self, id: &str, record: &C::Record) -> io::Result<PathBuf> {
        let document = C::encode(record).map_err(io::Error::other)?;
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path(id);
        let tmp = self.dir.join(format!(".{id}.tmp"));
        let result = std::fs::write(&tmp, document).and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(error) = result {
            let _ = std::fs::remove_file(&tmp);
            return Err(error);
        }
        Ok(path)
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        match std::fs::remove_file(self.path(id)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    pub fn mint(&self) -> String {
        std::iter::repeat_with(id::mint)
            .find(|id| !self.path(id).exists())
            .unwrap_or_else(id::mint)
    }
}

fn candidate<C: Codec>(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == C::EXT)
        && path
            .file_name()
            .is_some_and(|name| !name.to_string_lossy().starts_with('.'))
}

fn read<C: Codec>(path: &Path) -> Result<C::Record, String> {
    let id = path
        .file_stem()
        .ok_or_else(|| "no file name".to_string())?
        .to_string_lossy();
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    C::decode(&id, &text).map_err(|e| e.to_string())
}
