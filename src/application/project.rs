use std::io;
use std::path::{Path, PathBuf};

use super::{Entry, FileSystem, Store};
use crate::domain::{Annotation, Kind, Manifest};

pub struct Target {
    pub entry: Entry,
    pub(super) root: PathBuf,
    pub(super) relative: PathBuf,
}

pub enum Change {
    Updated,
    Unchanged,
    Cancelled,
}

pub struct Projects<F, S> {
    pub fs: F,
    pub store: S,
}

impl<F: FileSystem, S: Store> Projects<F, S> {
    pub fn init(&self, path: &Path) -> io::Result<usize> {
        let target = self.fs.resolve(path)?;
        if target.kind != Kind::Directory {
            return Err(io::Error::other("init requires a directory"));
        }
        let mut shadowed = 0;
        if let Some(parent) = target.path.parent() {
            match self.fs.project_root(parent) {
                Ok(root) => {
                    let relative = target.path.strip_prefix(&root).map_err(io::Error::other)?;
                    for path in self
                        .store
                        .read(&root)?
                        .keys()
                        .filter(|path| path.starts_with(relative))
                    {
                        if self.owner(&root, path)? == root {
                            shadowed += 1;
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        self.store.init(&target.path, &Manifest::new())?;
        Ok(shadowed)
    }

    pub fn target(&self, path: &Path) -> io::Result<Target> {
        let entry = self.fs.resolve(path)?;
        let start = if entry.kind == Kind::Directory {
            &entry.path
        } else {
            entry
                .path
                .parent()
                .ok_or_else(|| io::Error::other("missing parent directory"))?
        };
        let root = self.fs.project_root(start)?;
        let relative = entry.path.strip_prefix(&root).map_err(io::Error::other)?;
        let relative = if relative.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            relative.into()
        };
        Ok(Target {
            entry,
            root,
            relative,
        })
    }

    pub fn record(&self, target: &Target) -> io::Result<Option<Annotation>> {
        Ok(self.store.read(&target.root)?.remove(&target.relative))
    }

    pub fn list(
        &self,
        target: &Target,
        include: impl Fn(&Entry) -> bool,
    ) -> io::Result<Vec<(Entry, Option<Annotation>)>> {
        let manifest = self.store.read(&target.root)?;
        let mut entries = self.fs.entries(&target.entry.path)?;
        entries.sort_by(|a, b| {
            (a.kind != Kind::Directory, &a.path).cmp(&(b.kind != Kind::Directory, &b.path))
        });
        entries
            .into_iter()
            .filter(include)
            .map(|entry| {
                let relative = entry
                    .path
                    .strip_prefix(&target.root)
                    .map_err(io::Error::other)?;
                let record = if entry.kind == Kind::Directory
                    && self.fs.project_root(&entry.path)? == entry.path
                {
                    self.store.read(&entry.path)?.remove(Path::new("."))
                } else {
                    manifest.get(relative).cloned()
                };
                Ok((entry, record))
            })
            .collect()
    }

    pub fn set(
        &self,
        target: &Target,
        text: String,
        confirm: impl FnOnce(&str, &str) -> io::Result<bool>,
    ) -> io::Result<Change> {
        if text.contains(['\n', '\r']) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Comment must fit on one line",
            ));
        }
        let previous = self.record(target)?;
        if let Some(old) = &previous {
            if old.kind != target.entry.kind {
                return Err(io::Error::other("annotation type mismatch"));
            }
            if let Some(comment) = &old.comment {
                if comment == &text {
                    return Ok(Change::Unchanged);
                }
                if !confirm(comment, &text)? {
                    return Ok(Change::Cancelled);
                }
            }
        }
        let mut annotation = previous.clone().unwrap_or(Annotation {
            kind: target.entry.kind,
            comment: None,
            description: None,
        });
        annotation.comment = Some(text);
        self.replace(target, previous, Some(annotation))?;
        Ok(Change::Updated)
    }
}

impl<F: FileSystem, S: Store> Projects<F, S> {
    pub fn is_nested(&self, target: &Target, directory: &Path) -> io::Result<bool> {
        Ok(self.fs.project_root(directory)? != target.root)
    }

    pub fn edit(
        &self,
        target: &Target,
        edit: impl FnOnce(&Annotation) -> io::Result<Option<Annotation>>,
    ) -> io::Result<Change> {
        let previous = self.record(target)?;
        let initial = previous.clone().unwrap_or(Annotation {
            kind: target.entry.kind,
            comment: None,
            description: None,
        });
        if initial.kind != target.entry.kind {
            return Err(io::Error::other("annotation type mismatch"));
        }
        let Some(updated) = edit(&initial)? else {
            return Ok(Change::Cancelled);
        };
        if updated.kind != initial.kind
            || updated
                .comment
                .as_ref()
                .is_some_and(|line| line.contains(['\r', '\n']))
        {
            return Err(io::Error::other("invalid annotation"));
        }
        if updated == initial {
            return Ok(Change::Unchanged);
        }
        self.replace(target, previous, Some(updated))?;
        Ok(Change::Updated)
    }

    pub fn remove(&self, target: &Target) -> io::Result<bool> {
        let Some(previous) = self.record(target)? else {
            return Ok(false);
        };
        self.replace(target, Some(previous), None)?;
        Ok(true)
    }

    fn replace(
        &self,
        target: &Target,
        previous: Option<Annotation>,
        updated: Option<Annotation>,
    ) -> io::Result<()> {
        self.store.update(&target.root, |manifest| {
            let current = self.target(&target.entry.path)?;
            if current.root != target.root || current.entry.kind != target.entry.kind {
                return Err(io::Error::other(
                    "target changed while waiting; retry the command",
                ));
            }
            if manifest.get(&target.relative) != previous.as_ref() {
                return Err(io::Error::other(
                    "annotation changed while waiting; retry the command",
                ));
            }
            match updated {
                Some(record) if record.comment.is_some() || record.description.is_some() => {
                    manifest.insert(target.relative.clone(), record);
                }
                _ => {
                    manifest.remove(&target.relative);
                }
            }
            Ok(())
        })
    }
}
