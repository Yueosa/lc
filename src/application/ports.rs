use std::io;
use std::path::{Path, PathBuf};

use crate::domain::{Kind, Manifest};

pub struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
}

pub trait FileSystem {
    fn resolve(&self, path: &Path) -> io::Result<Entry>;
    fn project_root(&self, start: &Path) -> io::Result<PathBuf>;
    fn entries(&self, directory: &Path) -> io::Result<Vec<Entry>>;
}

pub trait Store {
    fn init(&self, root: &Path, manifest: &Manifest) -> io::Result<()>;
    fn read(&self, root: &Path) -> io::Result<Manifest>;
    fn update(
        &self,
        root: &Path,
        change: impl FnOnce(&mut Manifest) -> io::Result<()>,
    ) -> io::Result<()>;
}
