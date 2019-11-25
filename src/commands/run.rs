use failure::format_err;
use semver::VersionReq;

use crate::{
    shim,
    toolchain::{find_installed_toolchains, get_compatible_version_or_latest, InstalledToolchain},
    Result,
};

#[derive(Debug, failure::Fail)]
pub enum RunError {
    #[fail(display = "No interpreter found to run command: {}", _0)]
    MissingInterpreter(String),
}

pub fn run(version: Option<String>, command_and_args: &str) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .get(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", command_and_args))?;

    let installed_toolchains: Vec<InstalledToolchain> = find_installed_toolchains()?;
    let compatible_toolchain = get_compatible_version_or_latest(&installed_toolchains)?;

    match compatible_toolchain {
        Some(compatible_toolchain) => shim::run(&compatible_toolchain, cmd, arguments),
        None => {
            log::error!("No Python interpreter found at all. Please install at least one!");
            Err(RunError::MissingInterpreter(command_and_args.to_string()).into())
        }
    }
}
