use failure::format_err;
use semver::{Version, VersionReq};

use crate::{commands, config::Cfg, settings::Settings, Result};

mod compile;
mod download;

use self::{
    compile::{compile_source, extract_source},
    download::{download_source, find_all_python_versions},
};

pub fn run(
    from_version: Option<String>,
    cfg: &Option<Cfg>,
    settings: &Settings,
    install_extra_packages: &commands::InstallExtraPackagesOptions,
) -> Result<Option<Version>> {
    let version: VersionReq = match from_version {
        None => match cfg {
            None => Cfg::from_user_input()?.version,
            Some(cfg) => cfg.version.clone(),
        },
        Some(version) => VersionReq::parse(&version)?,
    };
    log::debug!("Installing Python {}", version);

    if settings
        .installed_python
        .iter()
        .any(|installed_python| version.matches(&installed_python.version))
    {
        log::info!("Python version {} already installed!", version);

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
        extract_source(&version_to_install)?;
        compile_source(&version_to_install, install_extra_packages)?;

        Ok(Some(version_to_install))
    }
}
