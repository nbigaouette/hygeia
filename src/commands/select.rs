use failure::format_err;
use semver::VersionReq;

use crate::{
    commands,
    installed::InstalledToolchain,
    selected::{SelectedVersion, VersionOrPath},
    utils, Result,
};

pub fn run(
    requested_version: commands::VersionOrPath,
    installed_toolchains: &[InstalledToolchain],
) -> Result<()> {
    log::debug!("Requested version: {:?}", requested_version);

    let version_or_path: VersionOrPath = requested_version.version_or_path.parse()?;

    let python_to_use = match version_or_path {
        VersionOrPath::VersionReq(version_req) => {
            match utils::active_version(&version_req, installed_toolchains) {
                Some(python_to_use) => python_to_use.clone(),
                None => {
                    return Err(format_err!(
                        "Python version {} not found!",
                        requested_version.version_or_path
                    ));
                }
            }
        }
        VersionOrPath::Path(path) => match InstalledToolchain::from_path(&path) {
            Some(python_to_use) => python_to_use,
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

    // Write to `.python-version`
    SelectedVersion {
        version: VersionReq::exact(&python_to_use.version),
    }
    .save()?;

    Ok(())
}
