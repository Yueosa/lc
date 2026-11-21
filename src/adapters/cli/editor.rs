use std::io::{self, Write};
use std::process::Command;

use crate::domain::Annotation;

pub fn edit(initial: &Annotation) -> io::Result<Option<Annotation>> {
    let mut draft = tempfile::Builder::new().suffix(".txt").tempfile()?;
    writeln!(
        draft,
        "Comment: {}",
        initial.comment.as_deref().unwrap_or("")
    )?;
    writeln!(draft, "Description:")?;
    if let Some(description) = &initial.description {
        write!(draft, "{description}")?;
    }
    draft.flush()?;
    let editor = ["VISUAL", "EDITOR"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "vi".into());
    let words = shlex::split(&editor).ok_or_else(|| io::Error::other("invalid editor command"))?;
    let (program, args) = words
        .split_first()
        .ok_or_else(|| io::Error::other("empty editor command"))?;
    if !Command::new(program)
        .args(args)
        .arg(draft.path())
        .status()?
        .success()
    {
        return Ok(None);
    }
    let content = std::fs::read_to_string(draft.path())?;
    let (comment, description) = content
        .split_once('\n')
        .ok_or_else(|| io::Error::other("expected Comment and Description headers"))?;
    let comment = comment
        .trim_end_matches('\r')
        .strip_prefix("Comment:")
        .ok_or_else(|| io::Error::other("expected Comment header"))?;
    let description = description
        .strip_prefix("Description:\n")
        .or_else(|| description.strip_prefix("Description:\r\n"))
        .ok_or_else(|| io::Error::other("expected Description header"))?;
    let comment = comment.strip_prefix(' ').unwrap_or(comment);
    Ok(Some(Annotation {
        kind: initial.kind,
        comment: (!comment.is_empty()).then(|| comment.to_owned()),
        description: (!description.is_empty()).then(|| description.to_owned()),
    }))
}
