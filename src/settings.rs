use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use log::{debug, error, warn};
use semver::Version;
use subprocess::{Exec, NullFile, Redirection};

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
    pub fn from_pycors_home() -> Result<Settings> {
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
                            error!("Error listing directory: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error parsing version string {:?}: {:?}", install_dir, e);
            }
        };

        let pycors_home_dir = utils::pycors_home()?;
        let bin_dir = pycors_home_dir.join("bin");

        // Find other Python installed (f.e. in system directories)
        let original_path = env::var("PATH")?;
        let other_pythons = get_python_versions_from_paths(&original_path, &bin_dir);
        installed_python.extend(other_pythons);

        Ok(Settings { installed_python })
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

    let mut other_pythons: HashMap<Version, PathBuf> = HashMap::new();
    let versions_sufix = &["", "2", "3"];

    for version_sufix in versions_sufix {
        let executable = format!("python{}", version_sufix);

        let python_path = match Exec::cmd("which")
            .arg(&executable)
            .stdout(Redirection::Pipe)
            .stderr(NullFile)
            .env("PATH", &path)
            .capture()
        {
            Err(e) => {
                error!(
                    "Failed to capture stdout from `which {}`: {:?}",
                    executable, e
                );
                continue;
            }
            Ok(python_path) => python_path,
        }
        .stdout_str();
        let python_path = python_path.trim();

        if !python_path.is_empty() {
            let python_path: &Path = path.as_ref();
            let python_pathbuf: PathBuf = python_path.to_path_buf();

            if python_path == skip_dir {
                debug!("Skipping pycors' own bin directory.");
                break;
            }

            debug!("Found python executable in {}", python_path.display());

            let full_executable_path = python_pathbuf.join(&executable);
            let python_version = match Exec::cmd(&full_executable_path)
                .arg("-V")
                .stdout(Redirection::Pipe)
                // Python 2 outputs its version to stderr, while python 3 to stdout.
                .stderr(Redirection::Merge)
                .capture()
            {
                Err(e) => {
                    error!(
                        "Failed to capture stdout from `which {}`: {:?}",
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
                    error!(
                        "Failed to parse output from `{} -V`: {}",
                        full_executable_path.display(),
                        python_version
                    );
                    continue;
                }
                Some(python_version_str) => python_version_str,
            };
            debug!("    {:?}", python_version_str);
            let python_version = match Version::parse(python_version_str) {
                Err(e) => {
                    error!(
                        "Failed to parse version string {:?}: {:?}",
                        python_version_str, e
                    );
                    continue;
                }
                Ok(python_version) => python_version,
            };
            debug!("    {:?}", python_version);

            other_pythons.insert(python_version, python_pathbuf);
        }
    }

    other_pythons
}
