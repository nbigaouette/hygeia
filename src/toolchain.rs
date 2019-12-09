use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use semver::{Version, VersionReq};
use thiserror::Error;
use which::which_in;

use crate::{
    constants::TOOLCHAIN_FILE,
    utils::{
        self,
        directory::{PycorsPaths, PycorsPathsFromEnv},
    },
    Result, EXECUTABLE_NAME,
};

pub mod installed;
pub mod selected;

use installed::{InstalledToolchain, NotInstalledToolchain};

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("Failed to get working current directory: {:?}", _0)]
    FailedCurrentDir(#[from] io::Error),
    #[error("Toolchain file {:?} is empty", _0)]
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
            Err(_) => {
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
            env::current_dir().map_err(ToolchainError::FailedCurrentDir)?;
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

#[derive(Debug)]
pub enum SelectedToolchain {
    InstalledToolchain(InstalledToolchain),
    NotInstalledToolchain(NotInstalledToolchain),
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

    pub fn from_toolchain_file(
        toolchain_file: &ToolchainFile,
        installed_toolchains: &[InstalledToolchain],
    ) -> SelectedToolchain {
        match toolchain_file {
            ToolchainFile::VersionReq(version_req) => {
                match find_compatible_toolchain(&version_req, &installed_toolchains) {
                    Some(compatible_toolchain) => {
                        SelectedToolchain::InstalledToolchain(compatible_toolchain.clone())
                    }
                    None => SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
                        version: Some(version_req.clone()),
                        location: None,
                    }),
                }
            }
            ToolchainFile::Path(path) => {
                let normalized_path = path.canonicalize();
                match normalized_path {
                    Ok(normalized_path) => SelectedToolchain::from_path(&normalized_path),
                    Err(e) => {
                        log::error!("Cannot use {:?} as toolchain path: {:?}", path, e);
                        log::error!(
                            "Please select a valid toolchain using: {} select",
                            EXECUTABLE_NAME
                        );
                        SelectedToolchain::NotInstalledToolchain(NotInstalledToolchain {
                            version: None,
                            location: Some(path.clone()),
                        })
                    }
                }
            }
        }
    }

    pub fn version_req(&self) -> Option<VersionReq> {
        match self {
            SelectedToolchain::InstalledToolchain(t) => Some(VersionReq::exact(&t.version)),
            SelectedToolchain::NotInstalledToolchain(t) => t.version.clone(),
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

    if !path.exists() {
        log::debug!("Skipping non-existing directory {}", path.display());
        return other_pythons;
    }

    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            log::error!("Failed to canonicalize path: {:?}", e);
            return other_pythons;
        }
    };

    let shims_dir = match PycorsPathsFromEnv::new().shims() {
        Ok(shims_dir) => shims_dir,
        Err(e) => {
            log::error!("Failed to get shims directory: {:?}", e);
            return other_pythons;
        }
    };
    let shims_dir = match shims_dir.canonicalize() {
        Ok(shims_dir) => shims_dir,
        Err(e) => {
            log::error!("Failed to canonicalize shims directory: {:?}", e);
            return other_pythons;
        }
    };
    if path == shims_dir {
        log::debug!("Skipping shims directory");
        return other_pythons;
    }

    for version_suffix in versions_suffix {
        let executable = format!("python{}", version_suffix);

        let full_executable_path = match which_in(&executable, Some(&path), "/") {
            Err(_) => {
                // log::debug!("Executable '{}' not found in {:?}", executable, path);
                continue;
            }
            Ok(python_path) => python_path,
        };
        let python_path = path.to_path_buf();

        let cmd_output = std::process::Command::new(&full_executable_path)
            .arg("-V")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute command: {:?}", full_executable_path));
        let python_version: String =
            match extract_version_from_command(&full_executable_path, cmd_output) {
                Ok(python_version) => python_version,
                Err(e) => {
                    log::error!("extract_version_from_command() failed: {:?}", e);
                    continue;
                }
            };

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
        log::debug!(
            "Found python executable in {}: {}",
            python_path.display(),
            python_version
        );

        other_pythons.insert(python_version, python_path);
    }

    other_pythons
}

fn extract_version_from_command(
    full_executable_path: &Path,
    output: Result<std::process::Output>,
) -> Result<String> {
    match output {
        Ok(output) => {
            if !output.status.success() {
                Err(anyhow::anyhow!(
                    "Failed to execute`{} -V` (exit code: {:?})",
                    full_executable_path.display(),
                    output.status.code()
                ))
            } else {
                match (
                    String::from_utf8(output.stdout),
                    String::from_utf8(output.stderr),
                ) {
                    (Ok(stdout), Ok(stderr)) => {
                        // Python 2 outputs its version to stderr, while python 3 to stdout.
                        Ok(format!("{}{}", stdout, stderr))
                    }
                    (Err(e), _) => Err(anyhow::anyhow!(
                        "Stdout of `{} -V` is not Utf-8: {:?}",
                        full_executable_path.display(),
                        e
                    )),
                    (_, Err(e)) => Err(anyhow::anyhow!(
                        "Stderr of `{} -V` is not Utf-8: {:?}",
                        full_executable_path.display(),
                        e
                    )),
                }
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to execute `{} -V`: {:?}",
            full_executable_path.display(),
            e
        )),
    }
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
    let install_dir = PycorsPathsFromEnv::new().installed()?;

    let mut installed_python = Vec::new();

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
                        // will be used.
                        let location_bin = PycorsPathsFromEnv::new().bin_dir(&version)?;
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
        for path in env::split_paths(&original_path) {
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

    compatible_versions.last().cloned()
}

pub enum CompatibleToolchainSource {
    File,
    String(String),
}

pub struct CompatibleToolchainBuilder {
    pick_latest_if_none_found: bool,
    load_from: CompatibleToolchainSource,
    overwrite: Option<VersionReq>,
}

impl CompatibleToolchainBuilder {
    pub fn new() -> CompatibleToolchainBuilder {
        CompatibleToolchainBuilder {
            pick_latest_if_none_found: false,
            load_from: CompatibleToolchainSource::File,
            overwrite: None,
        }
    }
    pub fn load_from_file(mut self) -> Self {
        self.load_from = CompatibleToolchainSource::File;
        self
    }
    pub fn load_from_string(mut self, source: &str) -> Self {
        self.load_from = CompatibleToolchainSource::String(source.to_string());
        self
    }
    pub fn pick_latest_if_none_found(mut self) -> Self {
        self.pick_latest_if_none_found = true;
        self
    }
    pub fn overwrite(mut self, with: Option<VersionReq>) -> Self {
        self.overwrite = with;
        self
    }
    pub fn compatible_version(self) -> Result<Option<InstalledToolchain>> {
        let installed_toolchains: Vec<InstalledToolchain> = find_installed_toolchains()?;

        let compatible = match self.overwrite {
            Some(version_req) => {
                log::info!("Overwriting version with {}", version_req);

                let search_result = find_compatible_toolchain(&version_req, &installed_toolchains);
                log::debug!("Compatible version found: {:?}", search_result);
                search_result
            }
            None => {
                // Load requested version from either .python-version (if present) or string
                let parsed_requested_toolchain: Option<ToolchainFile> = match &self.load_from {
                    CompatibleToolchainSource::File => {
                        let parsed: Option<ToolchainFile> = ToolchainFile::load()?.or_else(|| {
                            // We could not load a toolchain file.
                            log::warn!(
                                "File {:?} does not exists and could not be loaded.",
                                TOOLCHAIN_FILE
                            );
                            None
                        });
                        parsed
                    }
                    CompatibleToolchainSource::String(s) => {
                        let parsed = ToolchainFile::from_str(s)?;
                        Some(parsed)
                    }
                };

                let compatible: Option<&InstalledToolchain> = match parsed_requested_toolchain {
                    None => {
                        log::warn!("No compatible toolchain found.");

                        // No requested version (.python-version flag nor --version flag)
                        // Pick up the latest installed one (if asked for).
                        if self.pick_latest_if_none_found {
                            log::warn!("Trying latest installed...");
                            latest_installed(&installed_toolchains)
                        } else {
                            // We did not asked for a version (through the .python-version file
                            // or --version flag) and we did not asked to find the latest installed.
                            // We thus don't have any toolchain to run.
                            None
                        }
                    }
                    Some(requested_toolchain) => {
                        log::debug!("Searching for compatible toolchain in installed list...");

                        let selected_toolchain: SelectedToolchain =
                            SelectedToolchain::from_toolchain_file(
                                &requested_toolchain,
                                &installed_toolchains,
                            );

                        match selected_toolchain.version_req() {
                            Some(version_req) => {
                                log::debug!(
                                    "Searching for installed version compatible with: {}",
                                    version_req
                                );
                                let search_result =
                                    find_compatible_toolchain(&version_req, &installed_toolchains);
                                log::debug!("Compatible version found: {:?}", search_result);
                                search_result
                            }
                            None => {
                                log::warn!("Cannot find a compatible toolchain since selected toolchain is not installed.");

                                // We couldn't get a VersionReq from the toolchain, because we loaded a path
                                // from the toolchain file that does not contain a valid Python interpreter.
                                assert!(!selected_toolchain.is_installed());

                                if self.pick_latest_if_none_found {
                                    log::debug!("Finding latest installed one.");
                                    latest_installed(&installed_toolchains)
                                } else {
                                    // We asked for a specific version but couldn't find it and we did
                                    // not asked to find the latest installed.
                                    // We thus don't have any toolchain to run.
                                    None
                                }
                            }
                        }
                    }
                };
                compatible
            }
        };

        Ok(compatible.cloned())
    }
}

fn latest_installed(installed_toolchains: &[InstalledToolchain]) -> Option<&InstalledToolchain> {
    // We could not get a compatible version.
    // Let's pick the latest installed one instead, if any.
    let latest_toolchain: Option<&InstalledToolchain> = installed_toolchains.get(0);
    log::debug!(
        "Latest installed: {}",
        match latest_toolchain {
            None => String::from("None"),
            Some(t) => format!("{}", t),
        }
    );
    latest_toolchain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

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

    #[test]
    fn extract_version_from_command_success_py3() {
        let expected_version = String::from("Python 3.7.5");
        let output = Output {
            status: ExitStatus::from_raw(0),
            stdout: expected_version.as_bytes().to_vec(),
            stderr: b"".to_vec(),
        };
        let python_path = Path::new("/usr/local/python");
        let extracted_version = extract_version_from_command(&python_path, Ok(output)).unwrap();
        assert_eq!(extracted_version, expected_version);
    }

    #[test]
    fn extract_version_from_command_success_py2() {
        let expected_version = String::from("Python 2.7.10");
        let output = Output {
            status: ExitStatus::from_raw(0),
            stdout: b"".to_vec(),
            stderr: expected_version.as_bytes().to_vec(),
        };
        let python_path = Path::new("/usr/local/python2");
        let extracted_version = extract_version_from_command(&python_path, Ok(output)).unwrap();
        assert_eq!(extracted_version, expected_version);
    }
}
