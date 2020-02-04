// FIXME: Get rid of utils::path_exists(), use std::Path::exists() instead.
// FIXME: Replace 'format_err!()' with structs/enums
// FIXME: Gracefully handle errors that bubble to main
// FIXME: Add -vvv flag to control log level
// FIXME: Increase test coverage
// FIXME: Implement checksum/signature validation

use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    path::PathBuf,
};

use thiserror::Error;

use hygeia::{
    commands::{self, Command},
    constants::EXECUTABLE_NAME,
    shim, Opt, Result, StructOpt,
};

#[derive(Debug, Error)]
pub enum MainError {
    #[error("Cannot get executable's path: {0:?}")]
    Io(#[from] io::Error),
    #[error("Failed to get str representation of {0:?}")]
    Str(OsString),
    #[error("Cannot get executable's path: {0:?}")]
    ExecutablePath(PathBuf),
}

fn main() -> Result<()> {
    // Detect if running as shim as soon as possible
    let current_exe: PathBuf = env::current_exe().map_err(MainError::Io)?;
    let file_name: &OsStr = current_exe
        .file_name()
        .ok_or_else(|| MainError::ExecutablePath(current_exe.clone()))?;
    let exe = file_name
        .to_str()
        .ok_or_else(|| MainError::Str(file_name.to_os_string()))?;

    if exe.starts_with(EXECUTABLE_NAME) {
        no_shim_execution()
    } else {
        python_shim(exe)
    }
}

pub fn no_shim_execution() -> Result<()> {
    let opt = Opt::from_args();
    log::debug!("{:?}", opt);

    std::env::var("RUST_LOG").or_else(|_| -> Result<String> {
        let rust_log = format!("{}=info", EXECUTABLE_NAME);
        std::env::set_var("RUST_LOG", &rust_log);
        Ok(rust_log)
    })?;

    env_logger::init();

    if let Some(subcommand) = opt.subcommand {
        match subcommand {
            Command::List => commands::list::run()?,
            Command::Path { version } => commands::path::run(version)?,
            Command::Version { version } => commands::version::run(version)?,
            Command::Select(version_or_path) => commands::select::run(version_or_path)?,
            Command::Install {
                release,
                from_version,
                force,
                install_extra_packages,
                select,
            } => {
                commands::install::run(
                    release,
                    from_version,
                    force,
                    &install_extra_packages,
                    select,
                )?;
            }
            Command::Run { version, command } => commands::run::run(version, &command)?,
            Command::Setup { shell } => commands::setup::run(shell)?,
            #[cfg(feature = "self-update")]
            Command::Update => update()?,
        }
    }

    Ok(())
}

#[cfg(feature = "self-update")]
fn update() -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("nbigaouette")
        .repo_name(EXECUTABLE_NAME)
        .bin_name(&format!(
            "{}{}",
            EXECUTABLE_NAME,
            std::env::consts::EXE_SUFFIX
        ))
        .show_download_progress(true)
        .current_version(self_update::cargo_crate_version!())
        .build()?
        .update()?;
    println!("Update status: `{}`!", status.version());
    Ok(())
}

pub fn python_shim(command: &str) -> Result<()> {
    env_logger::init();

    let arguments: Vec<_> = env::args().collect();
    let (_, remaining_args) = arguments.split_at(1);

    shim::run(command, remaining_args)
}
