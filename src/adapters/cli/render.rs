use std::ffi::OsStr;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::domain::{Annotation, Kind};

pub fn name(name: &OsStr) -> String {
    let mut output = String::new();
    for chunk in name.as_bytes().utf8_chunks() {
        output.push_str(&text(&chunk.valid().replace('\\', "\\\\")));
        for byte in chunk.invalid() {
            use std::fmt::Write as _;
            write!(output, "\\x{byte:02x}").expect("writing to a String is infallible");
        }
    }
    output
}

pub fn text(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_control() {
            output.extend(character.escape_default());
        } else {
            output.push(character);
        }
    }
    output
}

pub fn comment(kind: Kind, record: Option<Annotation>) -> String {
    match record {
        Some(record) if record.kind == kind => {
            record.comment.map(|value| text(&value)).unwrap_or_default()
        }
        Some(_) => "[Mismatch]".into(),
        None => String::new(),
    }
}

pub fn show(
    out: &mut impl Write,
    path: &Path,
    kind: Kind,
    record: Option<Annotation>,
) -> io::Result<()> {
    writeln!(out, "Path         {}", name(path.as_os_str()))?;
    writeln!(out, "Type         {}", kind.label())?;
    if record.as_ref().is_some_and(|record| record.kind != kind) {
        return writeln!(out, "Status       Mismatch");
    }
    let (comment, description) = match record {
        Some(record) => (record.comment, record.description),
        None => (None, None),
    };
    writeln!(
        out,
        "\nComment\n  {}",
        text(comment.as_deref().unwrap_or("(not set)"))
    )?;
    writeln!(out, "\nDescription")?;
    for line in description.as_deref().unwrap_or("(not set)").split('\n') {
        writeln!(out, "  {}", text(line))?;
    }
    Ok(())
}
