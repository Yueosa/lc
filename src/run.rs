use std::process::ExitCode;

use liancomment::adapters::{cli, filesystem::LinuxFs, storage::JsonStore};
use liancomment::application::Projects;

pub fn run() -> ExitCode {
    let projects = Projects {
        fs: LinuxFs,
        store: JsonStore,
    };
    cli::run(&projects)
}
