use std::{
    collections::HashMap,
    env, fs,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use semver::Version;
use subprocess::{Exec, Redirection};
use which::which_in;

use crate::{utils, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct PythonVersion {
    pub location: PathBuf,
    pub version: Version,
}

impl PythonVersion {
    pub fn is_custom_install(&self) -> bool {
        match self.location.parent() {
            None => {
                log::error!("Cannot get parent directory of {:?}", self.location);
                false
            }
            Some(parent) => parent.join(crate::INFO_FILE).exists(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Settings {
    pub installed_python: Vec<PythonVersion>,

    // Prevent manual instantiation
    hidden: PhantomData<()>,
}

impl Settings {
    pub fn from_pycors_home() -> Result<Settings> {
        let install_dir = utils::pycors_installed()?;

        let mut installed_python = Vec::new();
        log::debug!("install_dir: {}", install_dir.display());

        match fs::read_dir(&install_dir) {
            Ok(dirs) => {
                for dir in dirs {
                    match dir {
                        Ok(dir) => {
                            let location = dir.path();
                            let version_str = match location.file_name() {
                                None => {
                                    log::error!(
                                        "Could not get the version from directory: {:?}",
                                        dir.path().display()
                                    );
                                    continue;
                                }
                                Some(dir) => match dir.to_str() {
                                    None => {
                                        log::error!(
                                            "Could not convert directory to str: {:?}",
                                            dir
                                        );
                                        continue;
                                    }
                                    Some(dir_str) => dir_str,
                                },
                            };

                            let version = match Version::parse(version_str.trim()) {
                                Err(e) => {
                                    log::error!(
                                        "Error parsing version string {:?}: {:?}",
                                        version_str.trim(),
                                        e
                                    );
                                    continue;
                                }
                                Ok(version) => version,
                            };

                            // Append `bin` to the path (if it exists) since this location
                            // will be used to call binaries directly.
                            let location_bin = location.join("bin");
                            let location = if location_bin.exists() {
                                location_bin
                            } else {
                                location
                            };

                            installed_python.push(PythonVersion { location, version });
                        }
                        Err(e) => {
                            log::error!("Error listing directory: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Error parsing version string {:?}: {:?}", install_dir, e);
            }
        };

        let pycors_home_dir = utils::config_home()?;
        let bin_dir = pycors_home_dir.join("bin");

        // Find other Python installed (f.e. in system directories)
        let original_path = env::var("PATH")?;
        let other_pythons = get_python_versions_from_paths(&original_path, &bin_dir);
        installed_python.extend(other_pythons);

        Ok(Settings {
            installed_python,
            hidden: PhantomData,
        })
    }
}

fn get_python_versions_from_paths<P>(original_path: &str, skip_dir: P) -> Vec<PythonVersion>
where
    P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>,
{
    let mut other_pythons: HashMap<Version, PathBuf> = HashMap::new();

    if !original_path.is_empty() {
        let paths = original_path.split(':');
        for path in paths {
            other_pythons.extend(get_python_versions_from_path(&path, &skip_dir));
        }
    }

    let mut other_pythons: Vec<PythonVersion> = other_pythons
        .into_iter()
        .map(|(version, location)| PythonVersion { location, version })
        .collect();
    other_pythons.sort_unstable_by(|p1, p2| p1.version.cmp(&p2.version));
    let other_pythons: Vec<PythonVersion> = other_pythons.into_iter().rev().collect();
    // debug!("Found extra versions:\n{:#?}", other_pythons);

    other_pythons
}

fn get_python_versions_from_path<P1, P2>(path: P1, skip_dir: P2) -> HashMap<Version, PathBuf>
where
    P1: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>,
    P2: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>,
{
    let skip_dir: &Path = skip_dir.as_ref();
    let path: &Path = path.as_ref();

    let mut other_pythons: HashMap<Version, PathBuf> = HashMap::new();
    let versions_suffix = &["", "2", "3"];

    for version_suffix in versions_suffix {
        let executable = format!("python{}", version_suffix);

        let python_path = match which_in(&executable, Some(&path), "/") {
            Err(_) => {
                // log::debug!("Executable '{}' not found in {:?}", executable, path);
                continue;
            }
            Ok(python_path) => python_path,
        };

        if python_path.exists() {
            let python_path: &Path = path;
            let python_pathbuf: PathBuf = python_path.to_path_buf();

            if python_path == skip_dir {
                log::debug!("Skipping pycors' own bin directory.");
                break;
            }

            log::debug!("python_path: {}", python_path.display());
            if python_path.join("pycors_dummy_file").exists() {
                log::debug!("Skipping pycors' shim directory.");
                break;
            }

            log::debug!("Found python executable in {}", python_path.display());

            let full_executable_path = python_pathbuf.join(&executable);
            let python_version = match Exec::cmd(&full_executable_path)
                .arg("-V")
                .stdout(Redirection::Pipe)
                // Python 2 outputs its version to stderr, while python 3 to stdout.
                .stderr(Redirection::Merge)
                .capture()
            {
                Err(e) => {
                    log::error!(
                        "Failed to capture stdout from `{}`: {:?}",
                        full_executable_path.display(),
                        e
                    );
                    continue;
                }
                Ok(python_version) => python_version,
            }
            .stdout_str();
            let python_version_str = match python_version.split_whitespace().nth(1) {
                None => {
                    log::error!(
                        "Failed to parse output from `{} -V`: {}",
                        full_executable_path.display(),
                        python_version
                    );
                    continue;
                }
                Some(python_version_str) => python_version_str,
            };
            log::debug!("    {:?}", python_version_str);
            let python_version = match Version::parse(python_version_str) {
                Err(e) => {
                    log::error!(
                        "Failed to parse version string {:?}: {:?}",
                        python_version_str,
                        e
                    );
                    continue;
                }
                Ok(python_version) => python_version,
            };
            log::debug!("    {:?}", python_version);

            other_pythons.insert(python_version, python_pathbuf);
        }
    }

    other_pythons
}
