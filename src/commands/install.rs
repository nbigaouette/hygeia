use std::{
    fs::File,
    io::{self, BufRead, Write},
    path::PathBuf,
};

use failure::format_err;
use semver::{Version, VersionReq};

use crate::{
    cache::AvailableToolchainsCache,
    commands::{
        self,
        install::{download::download_source, pip::install_extra_pip_packages},
    },
    constants::TOOLCHAIN_FILE,
    toolchain::{find_installed_toolchains, installed::InstalledToolchain, ToolchainFile},
    utils, Result,
};

mod download;
mod pip;
mod unix;
mod windows;

#[derive(Debug, failure::Fail)]
pub enum InstallError {
    #[fail(
        display = "Cannot install toolchain from file when specified as path ({:?})",
        _0
    )]
    ToolchainFileContainsPath(PathBuf),
}

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
    if matching_installed_version.is_none() || force_install {
        log::info!(
            "Installing Python {} (from {})",
            requested_version.version,
            requested_version.base_url
        );

        // Configure make make install
        download_source(&requested_version.version)?;
        // FIXME: Validate downloaded package with checksum
        // FIXME: Validate downloaded package with signature
        install_package(&requested_version.version, install_extra_packages)?;
    } else {
        log::warn!(
            "Python version {} already installed!",
            requested_version.version
        );
        log::warn!(
            "Compatible version found: {} (in {:?})",
            matching_installed_version.unwrap().version,
            matching_installed_version.unwrap().location,
        );
    }

    // Write .python-version file, if required
    if select {
        log::info!("Writing configuration to file {:?}", TOOLCHAIN_FILE);

        let version = format!("{}", VersionReq::exact(&requested_version.version));
        let mut output = File::create(&TOOLCHAIN_FILE)?;
        output.write(version.as_bytes())?;
        output.write(b"\n")?;
    }

    // Install extras
    let install_extra_flag_present = install_extra_packages.install_extra_packages
        || install_extra_packages.install_extra_packages_from.is_some();

    let install_dir = utils::directory::install_dir(&requested_version.version)?;

    if install_extra_flag_present {
        log::info!("Installing pip packages for {}", requested_version.version);

        install_extra_pip_packages(
            install_dir,
            &requested_version.version,
            install_extra_packages,
        )?;
    }

    Ok(())
}

fn install_package(
    version_to_install: &Version,
    install_extra_packages: &commands::InstallExtraPackagesOptions,
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
        None => return Err(format_err!("Standard input did not contain a single line")),
        Some(line_result) => line_result?,
    };
    log::debug!("Given: {}", line);

    let version: VersionReq = line.trim().parse()?;

    if line.is_empty() {
        log::error!("Empty line given as input.");
        Err(format_err!("Empty line provided"))
    } else {
        log::debug!("Parsed version: {}", version);
        Ok(version)
    }
}
