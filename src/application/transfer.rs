use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::{Kind, Manifest};

use super::{FileSystem, Projects, Status, Store, Target};

pub type Bundle = BTreeMap<PathBuf, Manifest>;

#[derive(Debug, PartialEq, Eq)]
pub struct ImportPlan {
    groups: BTreeMap<PathBuf, Manifest>,
    pub create: Vec<PathBuf>,
    pub update: Vec<PathBuf>,
    pub entries: usize,
    pub conflicts: usize,
    pub missing: usize,
    pub incompatible: usize,
}

impl<F: FileSystem, S: Store> Projects<F, S> {
    pub fn export(&self, target: &Target) -> io::Result<Bundle> {
        if target.entry.kind != Kind::Directory {
            return Err(io::Error::other("export requires a directory"));
        }
        let mut bundle = Bundle::new();
        let main = self
            .inspect(target, false)?
            .into_iter()
            .map(|(path, record, _)| {
                let path = if path == target.relative {
                    PathBuf::from(".")
                } else if target.relative == Path::new(".") {
                    path
                } else {
                    path.strip_prefix(&target.relative)
                        .expect("inspect returns scoped entries")
                        .to_owned()
                };
                (path, record)
            })
            .collect();
        bundle.insert(PathBuf::from("."), main);
        let mut pending = vec![target.entry.path.clone()];
        while let Some(directory) = pending.pop() {
            for entry in self.fs.entries(&directory)? {
                if entry.kind != Kind::Directory {
                    continue;
                }
                if self.fs.project_root(&entry.path)? == entry.path {
                    let nested = self.target(&entry.path)?;
                    let records = self
                        .inspect(&nested, true)?
                        .into_iter()
                        .map(|(path, record, _)| (path, record))
                        .collect();
                    let group = entry
                        .path
                        .strip_prefix(&target.entry.path)
                        .map_err(io::Error::other)?;
                    bundle.insert(group.to_owned(), records);
                }
                pending.push(entry.path);
            }
        }
        Ok(bundle)
    }

    pub fn plan_import(&self, target: &Target, bundle: &Bundle) -> io::Result<ImportPlan> {
        if target.entry.kind != Kind::Directory {
            return Err(io::Error::other("import requires a directory"));
        }
        for (group, records) in bundle {
            if !crate::domain::valid_relative(group)
                || records
                    .keys()
                    .any(|path| !crate::domain::valid_relative(path))
            {
                return Err(io::Error::other("invalid import path"));
            }
        }
        let mut plan = ImportPlan {
            groups: BTreeMap::new(),
            create: Vec::new(),
            update: Vec::new(),
            entries: 0,
            conflicts: 0,
            missing: 0,
            incompatible: 0,
        };
        let mut roots = BTreeMap::new();
        for (group, records) in bundle {
            let directory = if group == Path::new(".") {
                target.entry.path.clone()
            } else {
                target.entry.path.join(group)
            };
            let entry = match self.fs.resolve(&directory) {
                Ok(entry) if entry.path == directory && entry.kind == Kind::Directory => entry,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    plan.missing += records.len();
                    continue;
                }
                _ => {
                    plan.incompatible += records.len();
                    continue;
                }
            };
            let current = self.fs.project_root(&entry.path)?;
            let owner = if group == Path::new(".") {
                target.root.clone()
            } else {
                directory.clone()
            };
            // Existing ancestors between the import root and a proposed root must
            // also be represented by a bundle group.
            if current != owner && current != target.root && !roots.contains_key(&current) {
                plan.incompatible += records.len();
                continue;
            }
            roots.insert(directory, owner);
        }
        for (directory, owner) in &roots {
            let group = directory
                .strip_prefix(&target.entry.path)
                .map_err(io::Error::other)?;
            let group = if group.as_os_str().is_empty() {
                Path::new(".")
            } else {
                group
            };
            let existing = self.fs.project_root(directory)? == *owner;
            let current = if existing {
                self.store.read(owner)?
            } else {
                Manifest::new()
            };
            let mut additions = Manifest::new();
            for (path, record) in &bundle[group] {
                let absolute = if path == Path::new(".") {
                    directory.clone()
                } else {
                    directory.join(path)
                };
                let relative = absolute.strip_prefix(owner).map_err(io::Error::other)?;
                let relative = if relative.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    relative
                };
                let existing_owner = self.owner(owner, relative);
                match self.status(&absolute, record.kind) {
                    Status::Missing => {
                        plan.missing += 1;
                        continue;
                    }
                    Status::Valid => {}
                    _ => {
                        plan.incompatible += 1;
                        continue;
                    }
                }
                let crosses_planned_root = roots.keys().any(|root| {
                    root != directory && root.starts_with(directory) && absolute.starts_with(root)
                });
                if crosses_planned_root
                    || !existing_owner.is_ok_and(|actual| {
                        actual == *owner || !existing && owner.starts_with(&actual)
                    })
                {
                    plan.incompatible += 1;
                } else if current.contains_key(relative) {
                    plan.conflicts += 1;
                } else {
                    additions.insert(relative.to_owned(), record.clone());
                }
            }
            plan.entries += additions.len();
            if !existing {
                plan.create.push(owner.clone());
            } else if !additions.is_empty() {
                plan.update.push(owner.clone());
            }
            if !existing || !additions.is_empty() {
                plan.groups.insert(owner.clone(), additions);
            }
        }
        Ok(plan)
    }

    pub fn import(&self, target: &Target, bundle: &Bundle, plan: ImportPlan) -> io::Result<()> {
        if self.plan_import(target, bundle)? != plan {
            return Err(io::Error::other("import plan changed; review it again"));
        }
        for (root, records) in plan.groups {
            if plan.create.contains(&root) {
                self.store.init(&root, &records)?;
            } else if !records.is_empty() {
                self.store
                    .update(&root, |manifest| {
                        if records.keys().any(|path| manifest.contains_key(path)) {
                            return Err(io::Error::other("import conflict; retry to replan"));
                        }
                        manifest.extend(records);
                        Ok(())
                    })
                    .map_err(|error| {
                        io::Error::new(error.kind(), format!("{}: {error}", root.display()))
                    })?;
            }
        }
        Ok(())
    }
}
