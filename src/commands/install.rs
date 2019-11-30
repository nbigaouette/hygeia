use std::{
    fs::File,
    io::{self, BufRead, Write},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use semver::{Version, VersionReq};
use thiserror::Error;

use crate::{
    cache::AvailableToolchainsCache,
    commands::{self, install::download::download_source},
    constants::{EXECUTABLE_NAME, TOOLCHAIN_FILE},
    toolchain::{find_installed_toolchains, installed::InstalledToolchain, ToolchainFile},
};

mod download;
mod pip;
mod unix;
mod windows;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("Cannot install toolchain from file when specified as path ({0:?})")]
    ToolchainFileContainsPath(PathBuf),
}

// FIXME: Can't install the same version as an already installed system one.

pub fn run(
    requested_version: Option<String>,
    force_install: bool,
    install_extra_packages: &commands::InstallExtraPackagesOptions,
    select: bool,
) -> Result<()> {
    let requested_version_req: VersionReq = match requested_version {
        Some(requested_version) => {
            log::debug!("Parsing string {:?} as VersionReq", requested_version);
            if requested_version == "latest" {
                "*"
            } else {
                &requested_version
            }
            .parse()?
        }
        None => {
            log::warn!(
                "No version passed as argument, reading from file ({:?}).",
                TOOLCHAIN_FILE
            );
            match ToolchainFile::load()? {
                None => Ok(selected_version_from_user_input()?),
                Some(ToolchainFile::VersionReq(version_req)) => Ok(version_req),
                Some(ToolchainFile::Path(path)) => {
                    log::error!(
                        "Cannot install toolchain from file when specified as path ({:?})",
                        path
                    );
                    Err(InstallError::ToolchainFileContainsPath(path))
                }
            }?
        }
    };

    let cache = AvailableToolchainsCache::new()?;

    let requested_version = cache.query(&requested_version_req)?;

    // Already installed? Force installation?
    let installed_toolchains = find_installed_toolchains()?;
    let matching_installed_version: Option<&InstalledToolchain> =
        installed_toolchains.iter().find(|installed_python| {
            VersionReq::exact(&requested_version.version).matches(&installed_python.version)
                && installed_python.is_custom_install()
        });

    match (matching_installed_version, force_install) {
        (Some(matching_installed_version), false) => {
            log::warn!(
                "Python version {} already installed!",
                requested_version.version
            );
            log::warn!(
                "Compatible version found: {} (in {:?})",
                matching_installed_version.version,
                matching_installed_version.location,
            );
        }
        (_, true) | (None, _) => {
            log::info!(
                "Installing Python {} (from {})",
                requested_version.version,
                requested_version.base_url
            );

            // Install extras?
            let install_extra_packages: Option<&commands::InstallExtraPackagesOptions> =
                if install_extra_packages.install_extra_packages
                    || install_extra_packages.install_extra_packages_from.is_some()
                {
                    Some(install_extra_packages)
                } else {
                    None
                };

            // Configure make make install
            download_source(&requested_version.version)?;
            // FIXME: Validate downloaded package with checksum
            // FIXME: Validate downloaded package with signature
            install_package(&requested_version.version, install_extra_packages)?;
        }
    }

    // Write .python-version file, if required
    if select {
        log::info!("Writing configuration to file {:?}", TOOLCHAIN_FILE);

        let version = format!("{}", VersionReq::exact(&requested_version.version));
        let mut output = File::create(&TOOLCHAIN_FILE)?;
        output.write_all(version.as_bytes())?;
        output.write_all(b"\n")?;
    }

    log::info!("Installing {} succeeded!", requested_version.version);
    if select {
        log::info!(
            "Version {} is selected and will be used in current directory.",
            requested_version.version
        );
    } else {
        log::info!(
            "Version {} was installed but is not selected. Select it with:",
            requested_version.version
        );
        log::info!(
            "    {} select {}",
            EXECUTABLE_NAME,
            VersionReq::exact(&requested_version.version)
        );
    }

    Ok(())
}

fn install_package(
    version_to_install: &Version,
    install_extra_packages: Option<&commands::InstallExtraPackagesOptions>,
) -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    {
        unix::install_package(&version_to_install, install_extra_packages)?;
    }
    #[cfg(target_os = "windows")]
    {
        windows::install_package(&version_to_install, install_extra_packages)?;
    }

    Ok(())
}

fn selected_version_from_user_input() -> Result<VersionReq> {
    log::debug!("Reading configuration from stdin");

    let stdin = io::stdin();
    println!("Please type the Python version to use in this directory:");
    let line = match stdin.lock().lines().next() {
        None => return Err(anyhow!("Standard input did not contain a single line")),
        Some(line_result) => line_result?,
    };
    log::debug!("Given: {}", line);

    let version: VersionReq = line.trim().parse()?;

    if line.is_empty() {
        log::error!("Empty line given as input.");
        Err(anyhow!("Empty line provided"))
    } else {
        log::debug!("Parsed version: {}", version);
        Ok(version)
    }
}
