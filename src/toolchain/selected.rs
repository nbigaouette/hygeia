use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use failure::format_err;
use semver::VersionReq;

use crate::{constants::TOOLCHAIN_FILE, toolchain::installed::InstalledToolchain, utils, Result};

#[derive(Debug, PartialEq)]
pub enum VersionOrPath {
    VersionReq(semver::VersionReq),
    Path(PathBuf),
}

impl FromStr for VersionOrPath {
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
                log::info!("Parsed {:?} as semantic version : {}", s, version_req);
                Ok(VersionOrPath::VersionReq(version_req))
            }
            Err(e) => {
                log::debug!("e: {:?}", e);
                let path = Path::new(s);
                log::info!("Parsed {:?} as Path: {:?}", s, path);
                if path.exists() {
                    Ok(VersionOrPath::Path(
                        path.canonicalize().expect("path is expected to exists"),
                    ))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Path {:?} not found", s),
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectedVersion {
    pub version: VersionReq,
}

// FIXME: This function will now return a Some(Err(not installed)) is the required toolchain is not installed.
pub fn load_selected_toolchain_file(
    installed_toolchains: &[InstalledToolchain],
) -> Option<Result<InstalledToolchain>> {
    match env::current_dir() {
        Ok(mut path) => {
            loop {
                let toolchain_file = path.join(TOOLCHAIN_FILE);
                if utils::path_exists(&toolchain_file) {
                    // We've found the file, stop.
                    log::debug!("Found file {:?}", toolchain_file);
                    break Some(InstalledToolchain::from_select_file(
                        path,
                        installed_toolchains,
                    ));
                }

                if path.parent().is_none() {
                    // We are at the root directory, we haven't found anything.
                    break None;
                }

                path.pop();
            }
        }
        Err(e) => {
            log::error!("Failed to get current working directory: {:?}", e);
            Some(Err(e.into()))
        }
    }
}

impl SelectedVersion {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<SelectedVersion> {
        log::debug!("Reading configuration from file {:?}", path.as_ref());

        let input = File::open(path)?;
        let buffered = BufReader::new(input);

        // Read first line only
        let line = match buffered.lines().next() {
            None => return Err(format_err!("File does not even contains a line")),
            Some(line_result) => line_result?,
        };
        let version: VersionReq = line.parse()?;
        log::debug!("Found version \"{}\"", version);

        Ok(SelectedVersion { version })
    }

    pub fn save(&self) -> Result<usize> {
        self.save_to(TOOLCHAIN_FILE)
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<usize> {
        log::debug!("Writing configuration to file {:?}", path.as_ref());

        let version = format!("{}", self.version);
        let mut output = File::create(&path)?;
        let l1 = output.write(version.as_bytes())?;
        let l2 = output.write(b"\n")?;
        Ok(l1 + l2)
    }
}
