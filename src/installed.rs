use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use semver::Version;
use subprocess::{Exec, Redirection};
use which::which_in;

use crate::{constants::TOOLCHAIN_FILE, utils, Result, EXECUTABLE_NAME};

#[derive(Clone, Debug, PartialEq)]
pub struct InstalledToolchain {
    pub location: PathBuf,
    pub version: Version,
}

impl InstalledToolchain {
    pub fn from_path<P>(path: P) -> Option<InstalledToolchain>
    where
        P: AsRef<Path>,
    {
        let versions_found = get_python_versions_from_path(path.as_ref());
        log::debug!("versions_found: {:?}", versions_found);

        let highest_version = versions_found.into_iter().max_by(|x, y| (x.0.cmp(&y.0)))?;
        log::debug!("highest_version: {:?}", highest_version);

        Some(InstalledToolchain {
            version: highest_version.0,
            location: highest_version.1,
        })
    }

    pub fn is_custom_install(&self) -> bool {
        match self.location.parent() {
            None => {
                log::error!("Cannot get parent directory of {:?}", self.location);
                false
            }
            Some(parent) => parent.join(crate::INFO_FILE).exists(),
        }
    }

    pub fn save_version(&self) -> Result<usize> {
        let version = format!("{}", self.version);
        save(&version, TOOLCHAIN_FILE)
    }

    pub fn save_path(&self) -> Result<usize> {
        let location = format!("{}", self.location.display());
        save(&location, TOOLCHAIN_FILE)
    }
}

fn save<P>(content: &str, path: P) -> Result<usize>
where
    P: AsRef<Path>,
{
    log::debug!("Writing toolchain selection to file {:?}", path.as_ref());

    let mut output = File::create(&path)?;
    let l1 = output.write(content.as_bytes())?;
    let l2 = output.write(b"\n")?;
    Ok(l1 + l2)
}

pub fn find_installed_toolchains() -> Result<Vec<InstalledToolchain>> {
    let install_dir = utils::directory::installed()?;

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
                                    log::error!("Could not convert directory to str: {:?}", dir);
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
                        // will be used to call biInstalledToolchainly.
                        let location_bin = location.join("bin");
                        let location = if location_bin.exists() {
                            location_bin
                        } else {
                            location
                        };

                        installed_python.push(InstalledToolchain { location, version });
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

    // Find other Python installed (f.e. in system directories)
    let original_path = env::var("PATH")?;
    let other_pythons = get_python_versions_from_paths(&original_path);
    installed_python.extend(other_pythons);

    installed_python.sort_unstable_by(|p1, p2| p2.version.cmp(&p1.version));

    Ok(installed_python)
}

fn get_python_versions_from_paths(original_path: &str) -> Vec<InstalledToolchain> {
    let mut other_pythons: HashMap<Version, PathBuf> = HashMap::new();

    if !original_path.is_empty() {
        let paths = original_path.split(':');
        for path in paths {
            other_pythons.extend(get_python_versions_from_path(&path));
        }
    }

    let mut other_pythons: Vec<InstalledToolchain> = other_pythons
        .into_iter()
        .map(|(version, location)| InstalledToolchain { location, version })
        .collect();
    other_pythons.sort_unstable_by(|p1, p2| p1.version.cmp(&p2.version));
    let other_pythons: Vec<InstalledToolchain> = other_pythons.into_iter().rev().collect();
    // debug!("Found extra versions:\n{:#?}", other_pythons);

    other_pythons
}

fn get_python_versions_from_path<P>(path: P) -> HashMap<Version, PathBuf>
where
    P: AsRef<Path> + std::convert::AsRef<std::ffi::OsStr>,
{
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
            let python_path = path.to_path_buf();

            log::debug!("python_path: {}", python_path.display());
            if python_path.join(EXECUTABLE_NAME).exists() {
                log::debug!("Skipping {}' shim directory.", EXECUTABLE_NAME);
                break;
            }

            log::debug!("Found python executable in {}", python_path.display());

            let full_executable_path = python_path.join(&executable);
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

            other_pythons.insert(python_version, python_path);
        }
    }

    other_pythons
}
