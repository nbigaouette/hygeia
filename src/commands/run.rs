use failure::format_err;
use semver::VersionReq;

use crate::{selected::SelectedVersion, settings::InstalledToolchain, shim, utils, Result};

pub fn run(
    selected_version: &Option<SelectedVersion>,
    installed_toolchains: &[InstalledToolchain],
    version: Option<String>,
    command_and_args: &str,
) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .get(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", command_and_args))?;

    let interpreter_to_use = match version {
        None => utils::get_interpreter_to_use(selected_version, installed_toolchains)?,
        Some(version) => {
            let version_req = VersionReq::parse(&version)?;
            utils::active_version(&version_req, installed_toolchains)
                .ok_or_else(|| format_err!("Cannot find compatible version {}.", version))?
                .clone()
        }
    };

    shim::run(&interpreter_to_use, cmd, arguments)
}
