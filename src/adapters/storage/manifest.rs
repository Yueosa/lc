use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::application::{Bundle, Store};
use crate::domain::{Annotation, Kind, Manifest};

pub struct JsonStore;

#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    entries: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
struct Record {
    path: EncodedPath,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum EncodedPath {
    Text(String),
    Bytes { base64: String },
}

impl Store for JsonStore {
    fn init(&self, root: &Path, manifest: &Manifest) -> io::Result<()> {
        let temp = prepare(root, manifest)?;
        temp.persist_noclobber(root.join(".lc.json"))?;
        fs::File::open(root)?.sync_all()
    }

    fn read(&self, root: &Path) -> io::Result<Manifest> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(
                rustix::fs::OFlags::NOFOLLOW.bits() as i32
                    | rustix::fs::OFlags::NONBLOCK.bits() as i32,
            )
            .open(root.join(".lc.json"))?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other(
                "invalid .lc.json: expected a regular file",
            ));
        }
        let document: Document = serde_json::from_reader(file)?;
        document.into_manifest()
    }

    fn update(
        &self,
        root: &Path,
        change: impl FnOnce(&mut Manifest) -> io::Result<()>,
    ) -> io::Result<()> {
        let guard = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(
                rustix::fs::OFlags::NOFOLLOW.bits() as i32
                    | rustix::fs::OFlags::NONBLOCK.bits() as i32,
            )
            .open(root.join(".lc.lock"))?;
        if !guard.metadata()?.is_file() {
            return Err(io::Error::other(
                "invalid .lc.lock: expected a regular file",
            ));
        }
        guard.lock()?;
        let mut manifest = self.read(root)?;
        change(&mut manifest)?;
        prepare(root, &manifest)?.persist(root.join(".lc.json"))?;
        fs::File::open(root)?.sync_all()
    }
}

fn prepare(root: &Path, manifest: &Manifest) -> io::Result<NamedTempFile> {
    let mut temp = NamedTempFile::new_in(root)?;
    serde_json::to_writer_pretty(&mut temp, &Document::from_manifest(manifest))?;
    writeln!(&mut temp)?;
    temp.as_file().sync_all()?;
    Ok(temp)
}

impl Document {
    fn into_manifest(self) -> io::Result<Manifest> {
        if self.version != 1 {
            return Err(io::Error::other("unsupported manifest version"));
        }
        let mut manifest = Manifest::new();
        for record in self.entries {
            let path = record.path.into_path()?;
            let kind = match record.kind.as_str() {
                "File" => Kind::File,
                "Directory" => Kind::Directory,
                "Symlink" => Kind::Symlink,
                "Other" => Kind::Other,
                _ => return Err(io::Error::other("invalid annotation type")),
            };
            if manifest
                .insert(
                    path,
                    Annotation {
                        kind,
                        comment: record.comment,
                        description: record.description,
                    },
                )
                .is_some()
            {
                return Err(io::Error::other("duplicate manifest path"));
            }
        }
        Ok(manifest)
    }

    fn from_manifest(manifest: &Manifest) -> Self {
        let entries = manifest
            .iter()
            .map(|(path, annotation)| Record {
                path: EncodedPath::from_path(path),
                kind: annotation.kind.label().to_owned(),
                comment: annotation.comment.clone(),
                description: annotation.description.clone(),
            })
            .collect();
        Self {
            version: 1,
            entries,
        }
    }
}

impl EncodedPath {
    fn from_path(path: &Path) -> Self {
        match path.to_str() {
            Some(path) => Self::Text(path.to_owned()),
            None => Self::Bytes {
                base64: STANDARD.encode(path.as_os_str().as_bytes()),
            },
        }
    }

    fn into_path(self) -> io::Result<PathBuf> {
        let path = match self {
            Self::Text(path) => PathBuf::from(path),
            Self::Bytes { base64 } => PathBuf::from(std::ffi::OsString::from_vec(
                STANDARD.decode(base64).map_err(io::Error::other)?,
            )),
        };
        if !crate::domain::valid_relative(&path) {
            return Err(io::Error::other("invalid relative path"));
        }
        Ok(path)
    }
}

#[derive(Serialize, Deserialize)]
struct ExportDocument {
    version: u32,
    projects: Vec<ExportGroup>,
}

#[derive(Serialize, Deserialize)]
struct ExportGroup {
    root: EncodedPath,
    manifest: Document,
}

pub fn write_bundle(path: &Path, bundle: &Bundle) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent)?;
    let projects = bundle
        .iter()
        .map(|(root, records)| ExportGroup {
            root: EncodedPath::from_path(root),
            manifest: Document::from_manifest(records),
        })
        .collect();
    serde_json::to_writer_pretty(
        &mut temporary,
        &ExportDocument {
            version: 1,
            projects,
        },
    )?;
    writeln!(temporary)?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(path)?;
    Ok(())
}

pub fn read_bundle(path: &Path) -> io::Result<Bundle> {
    let document: ExportDocument = serde_json::from_reader(fs::File::open(path)?)?;
    if document.version != 1 {
        return Err(io::Error::other("unsupported export version"));
    }
    let mut bundle = Bundle::new();
    for group in document.projects {
        if bundle
            .insert(group.root.into_path()?, group.manifest.into_manifest()?)
            .is_some()
        {
            return Err(io::Error::other("duplicate project in export"));
        }
    }
    if !bundle.contains_key(Path::new(".")) {
        return Err(io::Error::other("missing main project in export"));
    }
    Ok(bundle)
}
