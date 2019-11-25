use failure::format_err;

use crate::{
    commands,
    toolchain::{
        find_installed_toolchains, installed::InstalledToolchain, selected::VersionOrPath,
        CompatibleToolchainBuilder,
    },
    utils, Result,
};

pub fn run(
    requested_version_or_path: commands::VersionOrPath,
    installed_toolchains: &[InstalledToolchain],
) -> Result<()> {
    log::debug!("Requested version: {:?}", requested_version_or_path);

    let version_or_path: VersionOrPath = requested_version_or_path.version_or_path.parse()?;

    let python_to_use: InstalledToolchain = match version_or_path {
        VersionOrPath::VersionReq(version_req) => {
            match utils::active_version(&version_req, installed_toolchains) {
                Some(python_to_use) => {
                    // Write to `.python-version`
                    python_to_use.save_version()?;

                    python_to_use.clone()
                }
                None => {
                    return Err(format_err!(
                        "Python version {} not found!",
                        requested_version_or_path.version_or_path
                    ));
                }
            }
        }
        VersionOrPath::Path(path) => match InstalledToolchain::from_path(&path) {
            Some(python_to_use) => {
                // Write to `.python-version`
                python_to_use.save_path()?;
                python_to_use
            }
            None => {
                return Err(format_err!(
                    "Could not find a Python interpreter under {:?}",
                    path
                ));
            }
        },
    };

    log::debug!(
        "Using {} from {}",
        python_to_use.version,
        python_to_use.location.display()
    );

    Ok(())
}
