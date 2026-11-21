use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::application::{Entry, FileSystem};
use crate::domain::Kind;

pub struct LinuxFs;

impl FileSystem for LinuxFs {
    fn resolve(&self, path: &Path) -> io::Result<Entry> {
        let absolute = std::path::absolute(path)?;
        let resolved = match (absolute.parent(), absolute.file_name()) {
            (Some(parent), Some(name)) => parent.canonicalize()?.join(name),
            _ => absolute.canonicalize()?,
        };
        let kind = kind(fs::symlink_metadata(&resolved)?.file_type());
        Ok(Entry {
            path: resolved,
            kind,
        })
    }

    fn project_root(&self, start: &Path) -> io::Result<PathBuf> {
        for directory in start.ancestors() {
            match fs::symlink_metadata(directory.join(".lc.json")) {
                Ok(metadata) if metadata.is_file() => return Ok(directory.to_owned()),
                Ok(_) => {
                    return Err(io::Error::other(
                        "invalid .lc.json: expected a regular file",
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no project found; run lc init",
        ))
    }

    fn entries(&self, directory: &Path) -> io::Result<Vec<Entry>> {
        fs::read_dir(directory)?
            .map(|entry| {
                let entry = entry?;
                Ok(Entry {
                    path: entry.path(),
                    kind: kind(entry.file_type()?),
                })
            })
            .collect()
    }
}

fn kind(kind: fs::FileType) -> Kind {
    if kind.is_dir() {
        Kind::Directory
    } else if kind.is_file() {
        Kind::File
    } else if kind.is_symlink() {
        Kind::Symlink
    } else {
        Kind::Other
    }
}
