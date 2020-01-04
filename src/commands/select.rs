use anyhow::{anyhow, Result};

use crate::{
    commands,
    toolchain::{
        find_installed_toolchains, installed::InstalledToolchain, selected::VersionOrPath,
    },
    utils::{self, directory::PycorsPathsProviderFromEnv},
};

pub fn run(requested_version_or_path: commands::VersionOrPath) -> Result<()> {
    log::debug!("Requested version: {:?}", requested_version_or_path);

    let paths_provider = PycorsPathsProviderFromEnv::new();
    let installed_toolchains = find_installed_toolchains(&paths_provider)?;

    let version_or_path: VersionOrPath = requested_version_or_path.version_or_path.parse()?;

    let python_to_use: InstalledToolchain = match version_or_path {
        VersionOrPath::VersionReq(version_req) => {
            match utils::active_version(&version_req, &installed_toolchains) {
                Some(python_to_use) => {
                    // Write to `.python-version`
                    python_to_use.save_version()?;

                    python_to_use.clone()
                }
                None => {
                    return Err(anyhow!(
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
                return Err(anyhow!(
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
