use std::io::{self, Write};
use std::path::Path;

use super::{prompt, render};
use crate::adapters::storage::{read_bundle, write_bundle};
use crate::application::{FileSystem, Projects, Status, Store, Target};

pub fn check<F: FileSystem, S: Store>(
    projects: &Projects<F, S>,
    target: &Target,
    out: &mut impl Write,
) -> io::Result<()> {
    let records = projects.inspect(target, false)?;
    writeln!(out, "Checked entries: {}", records.len())?;
    for status in [
        Status::Valid,
        Status::Missing,
        Status::Mismatch,
        Status::Unverified,
        Status::Unsupported,
    ] {
        writeln!(
            out,
            "{status:<12?} {}",
            records
                .iter()
                .filter(|(_, _, actual)| *actual == status)
                .count()
        )?;
    }
    Ok(())
}

pub fn clean<F: FileSystem, S: Store>(
    projects: &Projects<F, S>,
    target: &Target,
    all: bool,
    invalid_only: bool,
    out: &mut impl Write,
) -> io::Result<()> {
    if !invalid_only && target.entry.kind != crate::domain::Kind::Directory && !all {
        return Err(io::Error::other("clear requires a directory"));
    }
    let result = projects.cleanup(target, all, invalid_only, |scope, records| {
        if invalid_only {
            writeln!(out, "Invalid entries found: {}", records.len())?;
            for (path, _, status) in records {
                writeln!(out, "{status:?}\t{}", render::name(path.as_os_str()))?;
            }
            out.flush()?;
            prompt("Remove these invalid records? [y/N] ", false)
        } else {
            writeln!(
                out,
                "Target              {}",
                render::name(scope.as_os_str())
            )?;
            writeln!(out, "Annotated entries   {}", records.len())?;
            writeln!(
                out,
                "Comments            {}",
                records
                    .iter()
                    .filter(|(_, record, _)| record.comment.is_some())
                    .count()
            )?;
            writeln!(
                out,
                "Descriptions        {}",
                records
                    .iter()
                    .filter(|(_, record, _)| record.description.is_some())
                    .count()
            )?;
            writeln!(out, "This will permanently remove the listed annotations.")?;
            out.flush()?;
            prompt("Type \"confirm\" to continue: ", true)
        }
    })?;
    match result {
        Some(count) if invalid_only => writeln!(out, "Removed {count} invalid records."),
        Some(count) => writeln!(out, "Cleared {count} annotated entries."),
        None => writeln!(out, "Operation cancelled."),
    }
}

pub fn export<F: FileSystem, S: Store>(
    projects: &Projects<F, S>,
    target: &Target,
    output: &Path,
    out: &mut impl Write,
) -> io::Result<()> {
    let bundle = projects.export(target)?;
    write_bundle(output, &bundle)?;
    writeln!(
        out,
        "Entries exported: {}",
        bundle.values().map(|entries| entries.len()).sum::<usize>()
    )?;
    writeln!(out, "Output: {}", render::name(output.as_os_str()))
}

pub fn import<F: FileSystem, S: Store>(
    projects: &Projects<F, S>,
    target: &Target,
    input: &Path,
    out: &mut impl Write,
) -> io::Result<()> {
    let bundle = read_bundle(input)?;
    let plan = projects.plan_import(target, &bundle)?;
    writeln!(
        out,
        "Entries found:    {}",
        bundle.values().map(|entries| entries.len()).sum::<usize>()
    )?;
    writeln!(out, "New annotations:  {}", plan.entries)?;
    writeln!(out, "Conflicts:        {}", plan.conflicts)?;
    writeln!(out, "Missing targets:  {}", plan.missing)?;
    writeln!(out, "Incompatible:     {}", plan.incompatible)?;
    for (action, roots) in [("Create", &plan.create), ("Update", &plan.update)] {
        for root in roots {
            writeln!(
                out,
                "{action}: {}",
                render::name(root.join(".lc.json").as_os_str())
            )?;
        }
    }
    if plan.create.is_empty() && plan.update.is_empty() {
        return Ok(());
    }
    out.flush()?;
    if !prompt(
        &format!("Import {} new annotations? [y/N] ", plan.entries),
        false,
    )? {
        return writeln!(out, "Operation cancelled.");
    }
    let count = plan.entries;
    projects.import(target, &bundle, plan)?;
    writeln!(out, "Imported {count} entries.")
}
