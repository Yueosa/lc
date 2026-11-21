mod batch;
mod editor;
mod list;
mod render;

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::application::{Change, FileSystem, Projects, Store};
use crate::domain::Kind;

#[derive(Parser)]
#[command(
    version,
    about = "Manage project file annotations",
    subcommand_precedence_over_arg = true
)]
struct Args {
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Include hidden paths.
    #[arg(short = 'a')]
    all: bool,
    /// Show permissions, size and modification time.
    #[arg(short)]
    long: bool,
    /// Display a tree, stopping at nested projects.
    #[arg(short = 'R')]
    recursive: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize the current directory.
    Init,
    /// Set a one-line comment.
    Set {
        path: PathBuf,
        text: String,
        #[arg(long)]
        force: bool,
    },
    /// Print the comment for a path.
    Get { path: PathBuf },
    /// Show the comment and description for a path.
    Show { path: PathBuf },
    /// Edit Comment and Description in an external editor.
    Edit { path: PathBuf },
    /// Remove annotations for one path.
    Rm { path: PathBuf },
    /// Inspect recorded paths.
    Check { path: PathBuf },
    /// Remove confirmed missing or mismatched records.
    Clean { path: PathBuf },
    /// Clear annotations in a directory or the current project.
    Clear {
        #[arg(required_unless_present = "all")]
        path: Option<PathBuf>,
        #[arg(long, conflicts_with = "path")]
        all: bool,
    },
    /// Export annotations, including nested projects.
    Export {
        path: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Import an annotation bundle.
    Import {
        input: PathBuf,
        #[arg(long)]
        root: PathBuf,
    },
}

pub fn run<F: FileSystem, S: Store>(projects: &Projects<F, S>) -> ExitCode {
    match execute(Args::parse(), projects, &mut io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lc: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute<F: FileSystem, S: Store>(
    args: Args,
    projects: &Projects<F, S>,
    out: &mut impl Write,
) -> io::Result<()> {
    let path = match &args.command {
        Some(Command::Init) => Path::new("."),
        Some(
            Command::Set { path, .. }
            | Command::Get { path }
            | Command::Show { path }
            | Command::Edit { path }
            | Command::Rm { path }
            | Command::Check { path }
            | Command::Clean { path }
            | Command::Export { path, .. },
        ) => path,
        Some(Command::Clear { path, .. }) => path.as_deref().unwrap_or(Path::new(".")),
        Some(Command::Import { root, .. }) => root,
        None => &args.path,
    };
    let result = match &args.command {
        Some(Command::Init) => projects
            .init(path)
            .and_then(|shadowed| {
                if shadowed != 0 {
                    eprintln!("{shadowed} outer project record(s) are now shadowed and remain in the outer manifest.");
                }
                writeln!(out, "Initialized .lc.json")
            }),
        _ => execute_target(&args, path, projects, out),
    };
    result.map_err(|error| {
        if error.kind() == io::ErrorKind::BrokenPipe {
            error
        } else {
            io::Error::new(
                error.kind(),
                format!("{}: {error}", render::name(path.as_os_str())),
            )
        }
    })
}

fn execute_target<F: FileSystem, S: Store>(
    args: &Args,
    path: &Path,
    projects: &Projects<F, S>,
    out: &mut impl Write,
) -> io::Result<()> {
    let target = projects.target(path)?;
    match &args.command {
        Some(Command::Init) => unreachable!("init is handled before target resolution"),
        Some(Command::Set { text, force, .. }) => {
            let change = projects.set(&target, text.clone(), |old, new| {
                if *force { Ok(true) } else { confirm(old, new) }
            })?;
            match change {
                Change::Updated => {
                    writeln!(out, "Comment updated: {}", render::name(path.as_os_str()))
                }
                Change::Unchanged => Ok(()),
                Change::Cancelled => writeln!(out, "Operation cancelled."),
            }
        }
        Some(Command::Get { .. }) => {
            let record = projects.record(&target)?;
            let comment = match record {
                Some(record) if record.kind != target.entry.kind => {
                    return Err(io::Error::other("annotation type mismatch"));
                }
                Some(record) => record.comment,
                None => None,
            };
            if let Some(comment) = comment {
                writeln!(out, "{comment}")?;
            }
            Ok(())
        }
        Some(Command::Show { .. }) => {
            render::show(out, path, target.entry.kind, projects.record(&target)?)
        }
        None if target.entry.kind != Kind::Directory => {
            render::show(out, path, target.entry.kind, projects.record(&target)?)
        }
        Some(Command::Edit { .. }) => match projects.edit(&target, editor::edit)? {
            Change::Updated => writeln!(
                out,
                "Annotations updated: {}",
                render::name(path.as_os_str())
            ),
            Change::Unchanged => Ok(()),
            Change::Cancelled => writeln!(out, "Operation cancelled."),
        },
        Some(Command::Rm { .. }) => {
            if projects.remove(&target)? {
                writeln!(
                    out,
                    "Annotations removed: {}",
                    render::name(path.as_os_str())
                )?;
            }
            Ok(())
        }
        Some(Command::Check { .. }) => batch::check(projects, &target, out),
        Some(Command::Clean { .. }) => batch::clean(projects, &target, false, true, out),
        Some(Command::Clear { all, .. }) => batch::clean(projects, &target, *all, false, out),
        Some(Command::Export { output, .. }) => batch::export(projects, &target, output, out),
        Some(Command::Import { input, .. }) => batch::import(projects, &target, input, out),
        None => list::list(projects, &target, args.all, args.long, args.recursive, out),
    }
}

fn confirm(old: &str, new: &str) -> io::Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return Err(io::Error::other(
            "comment already exists; pass --force to overwrite",
        ));
    }
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "Old: {}", render::text(old))?;
    writeln!(stderr, "New: {}", render::text(new))?;
    drop(stderr);
    prompt("Overwrite? [y/N] ", false)
}

fn prompt(message: &str, literal: bool) -> io::Result<bool> {
    let mut stderr = io::stderr().lock();
    write!(stderr, "{message}")?;
    stderr.flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if literal {
        Ok(answer.trim_end_matches(['\r', '\n']) == "confirm")
    } else {
        Ok(matches!(answer.trim(), "y" | "Y"))
    }
}
