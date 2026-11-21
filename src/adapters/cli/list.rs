use std::io::{self, IsTerminal, Write};
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
use std::path::Path;

use chrono::{DateTime, Local};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::render;
use crate::application::{Entry, FileSystem, Projects, Store, Target};
use crate::domain::Kind;

pub fn list<F: FileSystem, S: Store>(
    projects: &Projects<F, S>,
    target: &Target,
    all: bool,
    long: bool,
    recursive: bool,
    out: &mut impl Write,
) -> io::Result<()> {
    if recursive {
        writeln!(
            out,
            "{}/",
            render::name(target.entry.path.as_os_str()).trim_end_matches('/')
        )?;
        return tree(projects, target, all, long, "", out);
    }
    let mut rows = Vec::new();
    for (entry, record) in projects.list(target, |entry| visible(entry, all))? {
        let label = label(&entry.path, entry.kind, long)?;
        let style = annotation_style(entry.kind, &record);
        rows.push((
            label,
            render::comment(entry.kind, record),
            entry.kind,
            style,
        ));
    }
    let heading = if long {
        "Permissions  Size       Modified          Name"
    } else {
        "Name"
    };
    let natural_width = rows
        .iter()
        .map(|(name, ..)| name.width())
        .max()
        .unwrap_or(0)
        .max(heading.width());
    let columns = terminal_width();
    let label_width = columns
        .map(|width| natural_width.min(width.saturating_sub(10).max(width / 2)))
        .unwrap_or(natural_width);
    let width = label_width + 2;
    let heading = truncate(heading, label_width);
    writeln!(
        out,
        "{heading}{}{}",
        " ".repeat(width - heading.width()),
        truncate(
            "Comment",
            columns
                .map(|columns| columns.saturating_sub(width))
                .unwrap_or(7)
        )
    )?;
    for (label, comment, kind, style) in rows {
        let comment = match columns {
            Some(columns) => truncate(&comment, columns.saturating_sub(width)),
            None => comment,
        };
        let label = truncate(&label, label_width);
        let padding = " ".repeat(width - label.width());
        let label = styled(&label, directory_style(kind));
        let comment = styled(&comment, style);
        writeln!(out, "{label}{padding}{comment}")?;
    }
    Ok(())
}

fn tree<F: FileSystem, S: Store>(
    projects: &Projects<F, S>,
    target: &Target,
    all: bool,
    long: bool,
    prefix: &str,
    out: &mut impl Write,
) -> io::Result<()> {
    let columns = terminal_width();
    let entries = projects.list(target, |entry| visible(entry, all))?;
    let mut entries = entries.into_iter().peekable();
    while let Some((entry, record)) = entries.next() {
        let last = entries.peek().is_none();
        let branch = if last { "└── " } else { "├── " };
        let directory = entry.kind == Kind::Directory;
        let nested = directory && projects.is_nested(target, &entry.path)?;
        let marker = if nested { " [project]" } else { "" };
        let label = label(&entry.path, entry.kind, long)?;
        let style = annotation_style(entry.kind, &record);
        let comment = render::comment(entry.kind, record);
        let comment = match columns {
            Some(columns) => truncate(
                &comment,
                columns.saturating_sub(
                    prefix.width() + branch.width() + label.width() + marker.width() + 2,
                ),
            ),
            None => comment,
        };
        let comment = styled(&comment, style);
        let label = styled(&label, directory_style(entry.kind));
        writeln!(out, "{prefix}{branch}{label}{marker}  {comment}")?;
        if directory && !nested {
            let child = projects.target(&entry.path)?;
            let next = format!("{prefix}{}", if last { "    " } else { "│   " });
            tree(projects, &child, all, long, &next, out)?;
        }
    }
    Ok(())
}

fn visible(entry: &Entry, all: bool) -> bool {
    all || !entry
        .path
        .file_name()
        .is_some_and(|name| name.as_bytes().starts_with(b"."))
}

fn terminal_width() -> Option<usize> {
    let stdout = io::stdout();
    stdout.is_terminal().then(|| {
        rustix::termios::tcgetwinsize(&stdout)
            .ok()
            .map(|window| usize::from(window.ws_col))
            .filter(|width| *width > 0)
            .or_else(|| {
                std::env::var("COLUMNS")
                    .ok()
                    .and_then(|value| value.parse().ok())
            })
            .unwrap_or(80)
    })
}

fn truncate(text: &str, max: usize) -> String {
    if text.width() <= max {
        return text.to_owned();
    }
    if max == 0 {
        return String::new();
    }
    let mut width = 0;
    let mut output = String::new();
    for character in text.chars() {
        width += character.width().unwrap_or(0);
        if width >= max {
            break;
        }
        output.push(character);
    }
    output.push('…');
    output
}

fn label(path: &Path, kind: Kind, long: bool) -> io::Result<String> {
    let name = render::name(path.file_name().unwrap_or(path.as_os_str()));
    let suffix = if kind == Kind::Directory { "/" } else { "" };
    if !long {
        return Ok(format!("{name}{suffix}"));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    let modified = DateTime::<Local>::from(metadata.modified()?).format("%Y-%m-%d %H:%M");
    let size = metadata.len();
    let size = if size >= 1024 * 1024 * 1024 {
        format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if size >= 1024 * 1024 {
        format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        format!("{:.1}K", size as f64 / 1024.0)
    } else {
        size.to_string()
    };
    Ok(format!(
        "{}   {size:<9}  {modified}  {name}{suffix}",
        permissions(kind, metadata.mode())
    ))
}

fn permissions(kind: Kind, mode: u32) -> String {
    let mut result = String::from(match kind {
        Kind::Directory => "d",
        Kind::Symlink => "l",
        Kind::File => "-",
        Kind::Other => "?",
    });
    for (index, character) in "rwxrwxrwx".chars().enumerate() {
        let bit = 8 - index;
        let ordinary = if mode & (1 << bit) != 0 {
            character
        } else {
            '-'
        };
        let special = match index {
            2 if mode & 0o4000 != 0 => {
                if ordinary == 'x' {
                    's'
                } else {
                    'S'
                }
            }
            5 if mode & 0o2000 != 0 => {
                if ordinary == 'x' {
                    's'
                } else {
                    'S'
                }
            }
            8 if mode & 0o1000 != 0 => {
                if ordinary == 'x' {
                    't'
                } else {
                    'T'
                }
            }
            _ => ordinary,
        };
        result.push(special);
    }
    result
}

fn annotation_style(kind: Kind, record: &Option<crate::domain::Annotation>) -> &'static str {
    if record.as_ref().is_some_and(|record| record.kind != kind) {
        "33"
    } else {
        "32"
    }
}

fn directory_style(kind: Kind) -> &'static str {
    if kind == Kind::Directory { "34" } else { "" }
}

fn styled(text: &str, style: &str) -> String {
    if text.is_empty()
        || style.is_empty()
        || !io::stdout().is_terminal()
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var_os("TERM").is_some_and(|term| term == "dumb")
    {
        return text.to_owned();
    }
    format!("\x1b[{style}m{text}\x1b[0m")
}
