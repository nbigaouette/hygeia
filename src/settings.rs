use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::PathBuf,
};

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
                            let version_file_path = location.join("version");
                            debug!("Loading version from {}", version_file_path.display());
                            let input = match File::open(&version_file_path) {
                                Err(e) => {
                                    error!("Error opening file {:?}: {:?}", version_file_path, e);
                                    continue;
                                }
                                Ok(input) => input,
                            };
                            let mut buffered = BufReader::new(input);

                            let mut buffer = String::new();
                            match buffered.read_to_string(&mut buffer) {
                                Err(e) => {
                                    error!("Error reading file {:?}: {:?}", version_file_path, e);
                                    continue;
                                }
                                Ok(_) => {}
                            }

                            let version = match Version::parse(buffer.trim()) {
                                Err(e) => {
                                    error!(
                                        "Error parsing version string {:?}: {:?}",
                                        buffer.trim(),
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
