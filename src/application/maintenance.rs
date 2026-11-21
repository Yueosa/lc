use std::io;
use std::path::{Path, PathBuf};

use crate::domain::{Annotation, Kind, Manifest};

use super::{FileSystem, Projects, Store, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Valid,
    Missing,
    Mismatch,
    Unverified,
    Unsupported,
}

impl<F: FileSystem, S: Store> Projects<F, S> {
    pub fn inspect(
        &self,
        target: &Target,
        all: bool,
    ) -> io::Result<Vec<(PathBuf, Annotation, Status)>> {
        let mut records = Vec::new();
        for (relative, record) in self.store.read(&target.root)? {
            let absolute = target.root.join(&relative);
            if !all
                && relative != target.relative
                && (target.entry.kind != Kind::Directory
                    || target.relative != Path::new(".") && !relative.starts_with(&target.relative))
            {
                continue;
            }
            match self.owner(&target.root, &relative) {
                Ok(owner) if owner != target.root => continue,
                Ok(_) => records.push((
                    relative,
                    record.clone(),
                    self.status(&absolute, record.kind),
                )),
                Err(error) => records.push((relative, record, status_error(&error))),
            }
        }
        Ok(records)
    }

    pub(super) fn owner(&self, root: &Path, relative: &Path) -> io::Result<PathBuf> {
        if relative == Path::new(".") {
            return self.fs.project_root(root);
        }
        let mut directory = root.to_owned();
        let mut owner = self.fs.project_root(root)?;
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            directory.push(component.as_os_str());
            match self.fs.resolve(&directory) {
                Ok(entry) if entry.path == directory && entry.kind == Kind::Directory => {
                    owner = self.fs.project_root(&directory)?;
                }
                Ok(_) if components.peek().is_none() => break,
                Ok(_) => return Err(io::Error::other("path crosses a non-directory or symlink")),
                Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                Err(error) => return Err(error),
            }
        }
        Ok(owner)
    }

    pub fn cleanup(
        &self,
        target: &Target,
        all: bool,
        invalid_only: bool,
        confirm: impl FnOnce(&Path, &[(PathBuf, Annotation, Status)]) -> io::Result<bool>,
    ) -> io::Result<Option<usize>> {
        let select = |rows: Vec<(PathBuf, Annotation, Status)>| {
            rows.into_iter()
                .filter(|(_, _, status)| {
                    !invalid_only || matches!(status, Status::Missing | Status::Mismatch)
                })
                .collect::<Vec<_>>()
        };
        let selected = select(self.inspect(target, all)?);
        if !invalid_only
            && selected
                .iter()
                .any(|(_, _, status)| matches!(status, Status::Unverified | Status::Unsupported))
        {
            return Err(io::Error::other("cannot determine the full cleanup scope"));
        }
        if selected.is_empty() {
            return Ok(Some(0));
        }
        let scope = if all {
            &target.root
        } else {
            &target.entry.path
        };
        if !confirm(scope, &selected)? {
            return Ok(None);
        }
        self.store.update(&target.root, |manifest: &mut Manifest| {
            if select(self.inspect(target, all)?) != selected {
                return Err(io::Error::other(
                    "cleanup scope changed; review the command again",
                ));
            }
            for (path, record, _) in &selected {
                if manifest.get(path) != Some(record) {
                    return Err(io::Error::other("annotation changed; retry the command"));
                }
                manifest.remove(path);
            }
            Ok(())
        })?;
        Ok(Some(selected.len()))
    }

    pub(super) fn status(&self, path: &Path, kind: Kind) -> Status {
        match self.fs.resolve(path) {
            Ok(entry) if entry.kind == kind => Status::Valid,
            Ok(_) => Status::Mismatch,
            Err(error) => status_error(&error),
        }
    }
}

fn status_error(error: &io::Error) -> Status {
    match error.kind() {
        io::ErrorKind::NotFound => Status::Missing,
        io::ErrorKind::Unsupported => Status::Unsupported,
        _ => Status::Unverified,
    }
}
