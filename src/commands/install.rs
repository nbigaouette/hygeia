use failure::format_err;
use semver::{Version, VersionReq};

use crate::{commands, installed::InstalledToolchain, selected::SelectedVersion, Result};

mod download;
mod pip;
mod unix;
mod windows;

use crate::commands::install::{
    download::{download_source, find_all_python_versions},
    pip::install_extra_pip_packages,
};

pub fn run(
    from_version: Option<String>,
    selected_version: &Option<SelectedVersion>,
    installed_toolchains: &[InstalledToolchain],
    install_extra_packages: &commands::InstallExtraPackagesOptions,
    select: bool,
) -> Result<Option<Version>> {
    let version: VersionReq = match from_version {
        None => match selected_version {
            None => SelectedVersion::from_user_input()?.version,
            Some(selected_version) => selected_version.version.clone(),
        },
        Some(version) => VersionReq::parse(&version)?,
    };
    log::debug!("Installing Python {}", version);

    let matching_installed_versions: Vec<_> = installed_toolchains
        .iter()
        .filter(|installed_python| {
            version.matches(&installed_python.version) && installed_python.is_custom_install()
        })
        .collect();

    if !matching_installed_versions.is_empty() {
        log::info!("Python version {} already installed!", version);
        log::info!(
            "Compatible versions found: {:?}",
            matching_installed_versions
        );

        let install_extra_flag_present = install_extra_packages.install_extra_packages
            || install_extra_packages.install_extra_packages_from.is_some();

        if install_extra_flag_present {
            // Safe to use `[0]` since we know for sure that vector is not-empty
            let first_matching_installed_versions = matching_installed_versions[0];
            log::info!(
                "Installing pip packages for toolchain {:?}",
                first_matching_installed_versions
            );

            let install_dir = &first_matching_installed_versions.location;
            install_extra_pip_packages(
                install_dir,
                &first_matching_installed_versions.version,
                install_extra_packages,
            )?;
        }

        if select {
            // Write to `.python-version`
            SelectedVersion { version }.save()?;
        }

        Ok(None)
    } else {
        // Get the last version compatible with the given version
        let versions = find_all_python_versions()?;
        let version_to_install = versions
            .into_iter()
            .find(|available_version| version.matches(&available_version))
            .ok_or_else(|| format_err!("Failed to find a compatible version to {}", version))?;
        log::info!("Found Python version {}", version_to_install);
        download_source(&version_to_install)?;
        install_package(&version_to_install, install_extra_packages)?;

        if select {
            // Write to `.python-version`
            SelectedVersion {
                version: VersionReq::exact(&version_to_install),
            }
            .save()?;
        }

        Ok(Some(version_to_install))
    }
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
