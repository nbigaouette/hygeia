use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    path::PathBuf,
};

use failure::format_err;
use git_testament::{git_testament, render_testament};
use human_panic::setup_panic;
use lazy_static::lazy_static;
use log::{debug, error};
use structopt::StructOpt;

mod commands;
mod constants;
mod dir_monitor;
mod installed;
mod os;
mod selected;
mod shim;
mod utils;

use crate::{
    commands::Command,
    constants::*,
    installed::{find_installed_toolchains, InstalledToolchain},
    selected::{load_selected_toolchain_file, SelectedVersion},
};

pub type Result<T> = std::result::Result<T, failure::Error>;

git_testament!(GIT_TESTAMENT);

fn git_version() -> &'static str {
    lazy_static! {
        static ref RENDERED: String = render_testament!(GIT_TESTAMENT);
    }
    &RENDERED
}

/// Control which Python toolchain to use on a directory basis.
#[derive(StructOpt, Debug)]
#[structopt(version = git_version())]
struct Opt {
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,

    #[structopt(subcommand)]
    subcommand: Option<commands::Command>,
}

#[derive(Debug, failure::Fail)]
pub enum MainError {
    #[fail(display = "Cannot get executable's path: {:?}", _0)]
    Io(#[fail(cause)] io::Error),
    #[fail(display = "Failed to get str representation of {:?}", _0)]
    Str(OsString),
    #[fail(display = "Cannot get executable's path: {:?}", _0)]
    ExecutablePath(PathBuf),
}
fn main() -> Result<()> {
    setup_panic!();

    // Detect if running as shim as soon as possible
    let current_exe: PathBuf = env::current_exe().map_err(|e| MainError::Io(e))?;
    let file_name: &OsStr = current_exe
        .file_name()
        .ok_or_else(|| MainError::ExecutablePath(current_exe.clone()))?;
    let exe = file_name
        .to_str()
        .ok_or_else(|| MainError::Str(file_name.to_os_string()))?;

    if exe.starts_with(EXECUTABLE_NAME) {
        // no_shim_execution(&selected_version_opt, &installed_toolchains)?;
        unimplemented!()
    } else {
        unimplemented!()
        // python_shim(
        //     exe,
        //     &selected_version_opt,
        //     &installed_toolchains,
        //     remaining_args,
        // )?;
    }

    //
    //
    //
    // ========================================================================
    //
    //
    //
    //
    //
    //
    //
    //

    // std::env::var("RUST_LOG").or_else(|_| -> Result<String> {
    //     let rust_log = format!("{}=warn", EXECUTABLE_NAME);
    //     std::env::set_var("RUST_LOG", &rust_log);
    //     Ok(rust_log)
    // })?;

    // env_logger::init();

    // let installed_toolchains = find_installed_toolchains()?;
    // // // Invert the Option<Result> to Result<Option> and use ? to unwrap the Result.
    // // let selected_version_opt =
    // //     load_selected_toolchain_file(&installed_toolchains).map_or(Ok(None), |v| v.map(Some))?;
    // let selected_version_opt = load_selected_toolchain_file(&installed_toolchains);

    // let arguments: Vec<_> = env::args().collect();
    // let (_, remaining_args) = arguments.split_at(1);
}

pub fn no_shim_execution(
    selected_version: &Option<SelectedVersion>,
    installed_toolchains: &[InstalledToolchain],
) -> Result<()> {
    let opt = Opt::from_args();
    log::debug!("{:?}", opt);

    if let Some(subcommand) = opt.subcommand {
        match subcommand {
            Command::Autocomplete { shell } => {
                commands::autocomplete::run(shell, &mut std::io::stdout())?;
            }
            Command::List => commands::list::run(selected_version, installed_toolchains)?,
            Command::Path => commands::path::run(selected_version, installed_toolchains)?,
            Command::Version => commands::version::run(selected_version, installed_toolchains)?,
            Command::Select(version_or_path) => {
                commands::select::run(version_or_path, installed_toolchains)?
            }
            Command::Install {
                from_version,
                install_extra_packages,
                select,
            } => {
                commands::install::run(
                    from_version,
                    selected_version,
                    installed_toolchains,
                    &install_extra_packages,
                    select,
                )?;
            }
            Command::Run { version, command } => {
                commands::run::run(selected_version, installed_toolchains, version, &command)?
            }
            Command::Setup { shell } => commands::setup::run(shell)?,
        }
    } else {
    }

    Ok(())
}

pub fn python_shim(
    command: &str,
    selected_version: &Option<SelectedVersion>,
    installed_toolchains: &[InstalledToolchain],
    arguments: &[String],
) -> Result<()> {
    let interpreter_to_use = utils::get_interpreter_to_use(selected_version, installed_toolchains)?;

    shim::run(&interpreter_to_use, command, arguments)
}

#[cfg(test)]
mod tests {

    // Version is reported as "unknown" in GitHub Actions.
    // See https://github.com/nbigaouette/pycors/pull/90/checks?check_run_id=311900597
    #[test]
    #[ignore]
    fn version() {
        let crate_version = structopt::clap::crate_version!();

        // GIT_VERSION is of the shape `v0.1.7-1-g095d7f5-modified`

        // Strip out the `v` prefix
        let (v, git_version_without_v) = crate::git_version().split_at(1);

        println!("crate_version: {:?}", crate_version);
        println!("v: {}", v);
        println!("git_version_without_v: {}", git_version_without_v);

        assert_eq!(v, "v");
        assert!(git_version_without_v.starts_with(crate_version));
    }
}
