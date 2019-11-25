use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use semver::{Version, VersionReq};
use subprocess::{Exec, Redirection};
use which::which_in;

use crate::{constants::TOOLCHAIN_FILE, utils, Result, EXECUTABLE_NAME};

// #[derive(Debug, PartialEq)]
// pub enum RequestedToolchain {
//     VersionReq(semver::VersionReq),
//     Path(PathBuf),
// }

#[derive(Debug, failure::Fail)]
pub enum ToolchainError {
    #[fail(display = "Failed to get working current directory: {:?}", _0)]
    FailedCurrentDir(#[fail(cause)] io::Error),
    #[fail(display = "Toolchain file {:?} is empty", _0)]
    EmptyToolchainFile(PathBuf),
}

#[derive(Debug, PartialEq)]
pub enum ToolchainFile {
    VersionReq(VersionReq),
    Path(PathBuf),
}

impl FromStr for ToolchainFile {
    type Err = std::io::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // One can use 'latest' to mean '*'
        if s == "latest" {
            "*"
        } else {
            s
        };

        match semver::VersionReq::parse(s) {
            Ok(version_req) => {
                log::info!("Parsed {:?} as semantic version: {}", s, version_req);
                Ok(ToolchainFile::VersionReq(version_req))
            }
            Err(e) => {
                let path = Path::new(s);
                log::info!("Parsed {:?} as Path: {:?}", s, path);
                if path.exists() {
                    Ok(ToolchainFile::Path(
                        path.canonicalize().expect("path is expected to exists"),
                    ))
                } else {
                    log::warn!("Requested path {:?} not found.", path);
                    Ok(ToolchainFile::Path(path.to_path_buf()))
                }
            }
        }
    }
}

impl ToolchainFile {
    pub fn load() -> Result<Option<ToolchainFile>> {
        let mut search_path: PathBuf =
            env::current_dir().map_err(|e| ToolchainError::FailedCurrentDir(e))?;
        let toolchain_file: Option<PathBuf> = loop {
            let toolchain_file: PathBuf = search_path.join(TOOLCHAIN_FILE);
            if utils::path_exists(&toolchain_file) {
                // We've found the file, stop.
                log::debug!("Found file {:?}", toolchain_file);
                break Some(toolchain_file);
            }

            if search_path.parent().is_none() {
                // We are at the root directory, we haven't found anything.
                break None;
            }

            search_path.pop();
        };

        let toolchain_file: Option<ToolchainFile> = match toolchain_file {
            None => None,
            Some(toolchain_file) => {
                log::debug!("Reading configuration from file {:?}", toolchain_file);

                let input = File::open(&toolchain_file)?;
                let buffered = BufReader::new(input);

                // Read first line only
                let line: String = match buffered.lines().next() {
                    Some(line_result) => line_result?,
                    None => return Err(ToolchainError::EmptyToolchainFile(toolchain_file).into()),
                };

                // Some(line.parse()?)
                match line.parse::<ToolchainFile>() {
                    Ok(parsed) => Some(parsed),
                    Err(e) => {
                        println!("e: {:?}", e);
                        unimplemented!()
                        // None
                        // Err(e)
                        // match e {
                        //     io
                        // }
                    }
                }
            }
        };

        Ok(toolchain_file)
    }
}

#[derive(Debug, Clone)]
pub struct InstalledToolchain {
    pub version: Version,
    pub location: PathBuf,
}

#[derive(Debug)]
pub struct NotInstalledToolchain {
    pub version: Option<VersionReq>,
    pub location: Option<PathBuf>,
}

#[derive(Debug)]
pub enum SelectedToolchain {
    InstalledToolchain(InstalledToolchain),
    NotInstalledToolchain(NotInstalledToolchain),
}

impl InstalledToolchain {
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

impl SelectedToolchain {
    pub fn from_path<P>(path: P) -> SelectedToolchain
    where
        P: AsRef<Path>,
    {
        let versions_found = get_python_versions_from_path(path.as_ref());
        log::debug!("Versions_found: {:?}", versions_found);

        match versions_found.into_iter().max_by(|x, y| (x.0.cmp(&y.0))) {
            None => {
                log::error!("No toolchain found in path {:?}", path.as_ref());
                SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
                    version: None,
                    location: Some(path.as_ref().to_path_buf()),
                })
            }
            Some(highest_version) => {
                log::debug!("Highest_version found in path: {:?}", highest_version);
                SelectedToolchain::InstalledToolchain(InstalledToolchain {
                    version: highest_version.0,
                    location: highest_version.1,
                })
            }
        }
    }

    pub fn is_installed(&self) -> bool {
        match self {
            SelectedToolchain::InstalledToolchain(_) => true,
            SelectedToolchain::NotInstalledToolchain(_) => false,
        }
    }

    pub fn same_version(&self, version: &VersionReq) -> bool {
        match self {
            SelectedToolchain::InstalledToolchain(t) => VersionReq::exact(&t.version) == *version,
            SelectedToolchain::NotInstalledToolchain(t) => match &t.version {
                None => false,
                Some(v) => v == version,
            },
        }
    }

    pub fn same_location(&self, location: &Path) -> bool {
        match self {
            SelectedToolchain::InstalledToolchain(t) => t.location == *location,
            SelectedToolchain::NotInstalledToolchain(t) => match &t.location {
                None => false,
                Some(p) => p == location,
            },
        }
    }
}

// FIXME: This does not need to be pub. When 'installed.rs' is finally deleted, make this private.
pub fn get_python_versions_from_path<P>(path: P) -> HashMap<Version, PathBuf>
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

pub fn is_a_custom_install<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    match path.parent() {
        None => {
            log::error!("Cannot get parent directory of {:?}", path);
            false
        }
        Some(parent) => parent.join(crate::INFO_FILE).exists(),
    }
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

pub fn find_compatible_toolchain<'a>(
    version_req: &VersionReq,
    installed_toolchains: &'a [InstalledToolchain],
) -> Option<&'a InstalledToolchain> {
    // Find all compatible versions from the installed list
    let mut compatible_versions: Vec<&'a InstalledToolchain> = installed_toolchains
        .iter()
        .filter(|installed_python| version_req.matches(&installed_python.version))
        .collect();
    // Sort to get latest version. If two versions are identical, pick the
    // one that is custom installed (not a system one).
    compatible_versions.sort_unstable_by(|a, b| {
        let version_comparison = a.version.cmp(&b.version);
        if version_comparison == std::cmp::Ordering::Equal {
            if a.is_custom_install() {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        } else {
            version_comparison
        }
    });
    log::debug!("Compatible versions found: {:?}", compatible_versions);

    compatible_versions.last().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_or_path_from_str_success_major_minor_patch() {
        let v = "3.7.4";
        let vop: ToolchainFile = v.parse().unwrap();
        assert_eq!(
            vop,
            ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
        );
    }
    #[test]
    fn version_or_path_from_str_success_eq_major_minor_patch() {
        let v = "=3.7.4";
        let vop: ToolchainFile = v.parse().unwrap();
        assert_eq!(
            vop,
            ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
        );
    }

    #[test]
    fn version_or_path_from_str_success_tilde_major_minor() {
        let v = "~3.7";
        let vop: ToolchainFile = v.parse().unwrap();
        assert_eq!(
            vop,
            ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
        );
    }

    #[test]
    fn version_or_path_from_str_success_tilde_major() {
        let v = "~3";
        let vop: ToolchainFile = v.parse().unwrap();
        assert_eq!(
            vop,
            ToolchainFile::VersionReq(VersionReq::parse(v).unwrap())
        );
    }
}
