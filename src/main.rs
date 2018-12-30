use std::{env, path::PathBuf};

use failure::format_err;
use log::{debug, error, warn};
use semver::VersionReq;

mod config;
mod download;
mod settings;
mod utils;

use crate::config::{load_config_file, Cfg};
use crate::settings::{load_settings_file, PythonVersion, Settings};

pub type Result<T> = std::result::Result<T, failure::Error>;

fn main() -> Result<()> {
    env_logger::init();

    match env::args().next() {
        None => {
            error!("Cannot get first argument.");
            Err(format_err!("Cannot get first argument"))?
        }
        Some(arg) => {
            let (mut settings, settings_file) = load_settings_file()?;
            let cfg = load_config_file()?;
            debug!("settings: {:?}", settings);
            debug!("cfg: {:?}", cfg);

            let python = select_python_version(&cfg.version, &settings.installed_python)?;

            if arg.ends_with("pycors") {
                debug!("Running pycors");
                pycors(&cfg, &mut settings, settings_file)
            } else {
                debug!("Running a Python shim");
                python_shim(&python)
            }
        }
    }
}

fn select_python_version(
    required_version: &VersionReq,
    installed_python: &[PythonVersion],
) -> Result<PythonVersion> {
    // Check if a compatible version is available. If not, download.utils
    let compatible = installed_python
        .iter()
        .find(|installed_python| required_version.matches(&installed_python.version));
    match compatible {
        None => {
            warn!("No compatible version found for {}", required_version);
            Err(format_err!("Not implemented"))
        }
        Some(compatible) => {
            debug!(
                "Found compatible installed version: {} (in {:?})",
                compatible.version, compatible.location
            );
            Ok(compatible.clone())
        }
    }
}

fn python_shim(python: &PythonVersion) -> Result<()> {
    unimplemented!()
}

fn pycors(cfg: &Cfg, settings: &mut Settings, settings_file: PathBuf) -> Result<()> {
    debug!("Saving settings to {:?}", settings_file);
    settings.save_to(settings_file)?;
    unimplemented!()
}
