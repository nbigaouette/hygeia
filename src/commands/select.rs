use failure::format_err;
use semver::VersionReq;

use crate::{
    commands,
    config::Cfg,
    settings::{PythonVersion, Settings},
    utils, Result,
};

pub fn run(
    requested_version: &str,
    settings: &Settings,
    install_extra_packages: &commands::InstallExtraPackagesOptions,
    install_if_not_present: bool,
) -> Result<()> {
    // Convert the requested version string to proper VersionReq
    // FIXME: Should a `~` be explicitly added here if user does not provide it?
    log::debug!("Requesting version: {}", requested_version);
    let version: VersionReq = requested_version.parse()?;
    log::debug!("Semantic version requirement: {}", version);

    let python_to_use = match utils::active_version(&version, settings) {
        Some(python_to_use) => python_to_use.clone(),
        None => {
            if install_if_not_present {
                let new_cfg = Some(Cfg { version });
                let version = commands::install::run(
                    None,
                    &new_cfg,
                    settings,
                    install_extra_packages,
                    false, // Don't 'select' here, will do so as last step.
                )?
                .ok_or_else(|| format_err!("A Python version should have been installed"))?;
                let install_dir = utils::directory::install_dir(&version)?;

                PythonVersion {
                    version,
                    location: install_dir,
                }
            } else {
                return Err(format_err!(
                    "Python version {} not found!",
                    requested_version
                ));
            }
        }
    };

    log::debug!(
        "Using {} from {}",
        python_to_use.version,
        python_to_use.location.display()
    );

    // Write to `.python-version`
    Cfg {
        version: VersionReq::exact(&python_to_use.version),
    }
    .save()?;

    Ok(())
}
