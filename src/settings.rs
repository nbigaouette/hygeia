use std::{fs, path::PathBuf};

use log::{debug, error, warn};
use semver::Version;

use crate::{utils, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct PythonVersion {
    pub location: PathBuf,
    pub version: Version,
}

#[derive(Debug, Default)]
pub struct Settings {
    pub installed_python: Vec<PythonVersion>,
}

impl Settings {
    pub fn new() -> Result<Settings> {
        let install_dir = utils::pycors_installed()?;

        let mut installed_python = Vec::new();
        debug!("install_dir: {}", install_dir.display());

        match fs::read_dir(&install_dir) {
            Ok(dirs) => {
                for dir in dirs {
                    match dir {
                        Ok(dir) => {
                            let location = dir.path();
                            let version_str = match location.file_name() {
                                None => {
                                    error!(
                                        "Could not get the version from directory: {:?}",
                                        dir.path().display()
                                    );
                                    continue;
                                }
                                Some(dir) => match dir.to_str() {
                                    None => {
                                        error!("Could not convert directory to str: {:?}", dir);
                                        continue;
                                    }
                                    Some(dir_str) => dir_str,
                                },
                            };

                            let version = match Version::parse(version_str.trim()) {
                                Err(e) => {
                                    error!(
                                        "Error parsing version string {:?}: {:?}",
                                        version_str.trim(),
                                        e
                                    );
                                    continue;
                                }
                                Ok(version) => version,
                            };

                            installed_python.push(PythonVersion { location, version });
                        }
                        Err(e) => {
                            error!("Error listing directory: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error parsing version string {:?}: {:?}", install_dir, e);
            }
        };

        Ok(Settings { installed_python })
    }
}
