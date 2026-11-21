use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Directory,
    Symlink,
    Other,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Directory => "Directory",
            Self::Symlink => "Symlink",
            Self::Other => "Other",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    pub kind: Kind,
    pub comment: Option<String>,
    pub description: Option<String>,
}

pub type Manifest = BTreeMap<PathBuf, Annotation>;

pub fn valid_relative(path: &std::path::Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();
    bytes == b"."
        || !bytes.contains(&0)
            && bytes
                .split(|byte| *byte == b'/')
                .all(|part| !part.is_empty() && part != b"." && part != b"..")
}
